//! Open a trusted supervisor binary and execute that file descriptor.
//!
//! The check and the exec use the same descriptor (`O_PATH|O_NOFOLLOW`, then
//! `execveat(AT_EMPTY_PATH)`), so a path swap after the check cannot change
//! what runs. Parent directories are opened the same way, one component at a
//! time, so a symlink above the file is rejected too.

use std::ffi::{CStr, CString, OsStr, OsString};
use std::io;
use std::os::fd::{AsRawFd, BorrowedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::{Component, Path};
use std::process::{Child, Command};

use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, FileType, Mode, OFlags};

const DIR_FLAGS: OFlags = OFlags::PATH
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const FILE_FLAGS: OFlags = OFlags::PATH.union(OFlags::NOFOLLOW).union(OFlags::CLOEXEC);

pub(crate) fn open_trusted_supervisor(path: &Path) -> io::Result<OwnedFd> {
    if !path.is_absolute() {
        return Err(io::Error::other("supervisor path must be absolute"));
    }
    let mut names = Vec::new();
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => names.push(name.to_os_string()),
            Component::CurDir | Component::ParentDir | Component::Prefix(_) => {
                return Err(io::Error::other("supervisor path must not contain . or .."));
            }
        }
    }
    let Some(file_name) = names.pop() else {
        return Err(io::Error::other("supervisor path is missing a file name"));
    };

    let root = rustix::fs::open("/", DIR_FLAGS, Mode::empty()).map_err(io::Error::from)?;
    trusted_directory(&root)?;
    let mut current = root;
    for name in &names {
        current = rustix::fs::openat(&current, name.as_os_str(), DIR_FLAGS, Mode::empty())
            .map_err(io::Error::from)?;
        trusted_directory(&current)?;
    }
    let file = rustix::fs::openat(&current, file_name.as_os_str(), FILE_FLAGS, Mode::empty())
        .map_err(io::Error::from)?;
    trusted_file(&file)?;
    Ok(file)
}

pub(crate) fn owner_is_trusted(owner: u32) -> bool {
    owner == 0 || owner == rustix::process::getuid().as_raw()
}

fn trusted_directory(fd: &OwnedFd) -> io::Result<()> {
    let stat = rustix::fs::fstat(fd).map_err(io::Error::from)?;
    if !FileType::from_raw_mode(stat.st_mode).is_dir() {
        return Err(io::Error::other(
            "supervisor parent directory is not a directory",
        ));
    }
    if !owner_is_trusted(stat.st_uid) {
        return Err(io::Error::other(
            "supervisor parent directory is not owned by the current user or root",
        ));
    }
    let mode = Mode::from_raw_mode(stat.st_mode);
    let group_or_other_write = mode.contains(Mode::WGRP) || mode.contains(Mode::WOTH);
    let root_sticky = stat.st_uid == 0 && mode.contains(Mode::SVTX);
    if group_or_other_write && !root_sticky {
        return Err(io::Error::other(
            "supervisor parent directory is writable by group or other",
        ));
    }
    Ok(())
}

fn trusted_file(fd: &OwnedFd) -> io::Result<()> {
    let stat = rustix::fs::fstat(fd).map_err(io::Error::from)?;
    if !FileType::from_raw_mode(stat.st_mode).is_file() {
        return Err(io::Error::other("supervisor is not a regular file"));
    }
    if !owner_is_trusted(stat.st_uid) {
        return Err(io::Error::other(
            "supervisor is not owned by the current user or root",
        ));
    }
    let mode = Mode::from_raw_mode(stat.st_mode);
    if mode.contains(Mode::WGRP) || mode.contains(Mode::WOTH) {
        return Err(io::Error::other("supervisor is writable by group or other"));
    }
    Ok(())
}

/// Replace the forked child with the opened supervisor. `template` supplies
/// the working directory and environment of the bash command.
#[allow(unsafe_code)]
pub(crate) fn spawn_trusted(
    fd: &OwnedFd,
    args: &[OsString],
    template: &Command,
) -> io::Result<Child> {
    let mut argument_strings = Vec::with_capacity(args.len());
    for arg in args {
        argument_strings.push(cstring_from_os(arg)?);
    }
    let envp = environment_from(template)?;
    let raw = fd.as_raw_fd();
    let mut command = Command::new("/bin/sh");
    if let Some(dir) = template.get_current_dir() {
        command.current_dir(dir);
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    // Safety: `pre_exec` runs in the forked child. `execveat` replaces that
    // child with the inode we already fstat'd, or returns without running
    // `/bin/sh`. The descriptor was opened `O_CLOEXEC` and is still open
    // across `fork`.
    unsafe {
        command.pre_exec(move || {
            let argv_ptrs = cstring_ptrs(&argument_strings);
            let env_ptrs = cstring_ptrs(&envp);
            // Safety: `raw` is the trusted descriptor inherited across `fork`.
            // `execveat(AT_EMPTY_PATH)` replaces this process with that inode,
            // so `/bin/sh` never runs. The pointer arrays are NUL-terminated
            // and live until the call returns. This call sits in the `unsafe`
            // block that invokes `pre_exec`.
            let err = rustix::runtime::execveat(
                BorrowedFd::borrow_raw(raw),
                empty_path(),
                argv_ptrs.as_ptr(),
                env_ptrs.as_ptr(),
                AtFlags::EMPTY_PATH,
            );
            Err(io::Error::from(err))
        });
    }
    command.spawn()
}

/// Run the shell in a new session, and move it into `cgroup_procs` before exec.
#[allow(unsafe_code)]
pub(crate) fn isolate_shell(command: &mut Command, cgroup_procs: &Path) {
    let procs = cgroup_procs.to_path_buf();
    // Safety: runs after `fork` and before `exec`. Failure aborts the exec,
    // so the command cannot start outside the cgroup or in our session.
    unsafe {
        command.pre_exec(move || {
            rustix::process::setsid().map_err(io::Error::from)?;
            write_self_to_cgroup(&procs)?;
            Ok(())
        });
    }
}

/// New session only. Used for the command inside a pid namespace.
#[allow(unsafe_code)]
pub(crate) fn setsid_child(command: &mut Command) {
    unsafe {
        command.pre_exec(|| {
            rustix::process::setsid().map_err(io::Error::from)?;
            Ok(())
        });
    }
}

fn write_self_to_cgroup(procs: &Path) -> io::Result<()> {
    let pid = rustix::process::getpid().as_raw_pid();
    let mut buf = [0_u8; 16];
    let length = format_pid(pid, &mut buf);
    let fd = rustix::fs::open(procs, OFlags::WRONLY | OFlags::CLOEXEC, Mode::empty())
        .map_err(io::Error::from)?;
    write_all(&fd, &buf[..length])
}

fn write_all(fd: &OwnedFd, mut buf: &[u8]) -> io::Result<()> {
    while !buf.is_empty() {
        let wrote = rustix::io::write(fd, buf).map_err(io::Error::from)?;
        if wrote == 0 {
            return Err(io::Error::other("cgroup.procs write returned no bytes"));
        }
        buf = &buf[wrote..];
    }
    Ok(())
}

fn format_pid(pid: i32, buf: &mut [u8; 16]) -> usize {
    let mut value = pid.unsigned_abs();
    let mut digits = [0_u8; 16];
    let mut count = 0;
    if value == 0 {
        digits[0] = b'0';
        count = 1;
    } else {
        while value > 0 {
            digits[count] = b'0' + u8::try_from(value % 10).unwrap_or(0);
            value /= 10;
            count += 1;
        }
    }
    for (slot, digit) in buf.iter_mut().zip(digits[..count].iter().rev()) {
        *slot = *digit;
    }
    count
}

fn cstring_from_os(value: &OsStr) -> io::Result<CString> {
    CString::new(value.as_bytes())
        .map_err(|_| io::Error::other("supervisor argument contains a NUL"))
}

fn environment_from(template: &Command) -> io::Result<Vec<CString>> {
    let mut env = std::env::vars_os().collect::<Vec<_>>();
    for (key, value) in template.get_envs() {
        env.retain(|(existing, _)| existing != key);
        if let Some(value) = value {
            env.push((key.to_os_string(), value.to_os_string()));
        }
    }
    let mut encoded = Vec::with_capacity(env.len());
    for (key, value) in env {
        if key.as_bytes().contains(&0) || value.as_bytes().contains(&0) {
            return Err(io::Error::other("environment contains a NUL byte"));
        }
        let mut bytes = key.as_bytes().to_vec();
        bytes.push(b'=');
        bytes.extend_from_slice(value.as_bytes());
        encoded.push(
            CString::new(bytes).map_err(|_| io::Error::other("environment contains a NUL byte"))?,
        );
    }
    Ok(encoded)
}

fn cstring_ptrs(values: &[CString]) -> Vec<*const u8> {
    let mut ptrs = values
        .iter()
        .map(|value| value.as_ptr().cast::<u8>())
        .collect::<Vec<_>>();
    ptrs.push(std::ptr::null());
    ptrs
}

fn empty_path() -> &'static CStr {
    c""
}
