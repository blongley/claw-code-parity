//! Contain one timed command in a pid namespace created by this process.
//!
//! `clone3(CLONE_NEWUSER|CLONE_NEWPID|CLONE_PIDFD)` returns the init pidfd in
//! the same syscall that creates the child, so teardown never reopens a pid
//! that might have been reused. The child arms `PR_SET_PDEATHSIG` and checks
//! `getppid` before it writes one ack byte. The parent writes the release byte
//! only after that ack. Timeout and teardown are `pidfd_send_signal(SIGKILL)`
//! and `waitid(P_PIDFD)` on the fd clone3 returned.

#![allow(unsafe_code)]

#[cfg(test)]
use std::cell::Cell;
use std::ffi::{CString, OsStr};
use std::io::{self, Read};
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec};
use rustix::io::Errno;
use rustix::process::{Signal, WaitId, WaitIdOptions, WaitIdStatus, WaitOptions, WaitStatus};

/// Status text for a process that survived teardown. Not an exit code.
pub(crate) const UNKILLABLE_STATUS: &str = "processes could not be killed";

const RELEASED: u8 = 1;
const ARM_FAILED: i32 = 86;
const EMPTY_WAIT: Duration = Duration::from_secs(2);
/// Allowlisted child descriptors. `close_range(3, ~0, 0)` drops every other fd
/// the child inherited, including another init's stdout writer.
const FD_CONTROL: i32 = 0;
const FD_STDOUT: i32 = 1;
const FD_STDERR: i32 = 2;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Containment {
    Finished(i32),
    TimedOut,
    Unkillable { detail: String },
    Unavailable { detail: String },
    Failed(String),
}

pub(crate) struct Captured {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[cfg(test)]
thread_local! {
    static FAIL_NAMESPACE: Cell<bool> = const { Cell::new(false) };
    static FAIL_ARMING: Cell<bool> = const { Cell::new(false) };
    static RELEASES: Cell<u32> = const { Cell::new(0) };
}

#[cfg(test)]
#[must_use = "the injected failure ends when this guard is dropped"]
pub(crate) struct FailureGuard {
    kind: FailureKind,
}

#[cfg(test)]
#[derive(Clone, Copy)]
enum FailureKind {
    Namespace,
    Arming,
}

#[cfg(test)]
pub(crate) fn inject_namespace_failure() -> FailureGuard {
    FAIL_NAMESPACE.with(|cell| cell.set(true));
    FailureGuard {
        kind: FailureKind::Namespace,
    }
}

#[cfg(test)]
pub(crate) fn inject_arming_failure() -> FailureGuard {
    FAIL_ARMING.with(|cell| cell.set(true));
    FailureGuard {
        kind: FailureKind::Arming,
    }
}

#[cfg(test)]
impl Drop for FailureGuard {
    fn drop(&mut self) {
        match self.kind {
            FailureKind::Namespace => FAIL_NAMESPACE.with(|cell| cell.set(false)),
            FailureKind::Arming => FAIL_ARMING.with(|cell| cell.set(false)),
        }
    }
}

#[cfg(test)]
pub(crate) fn release_count() -> u32 {
    RELEASES.with(Cell::get)
}

pub(crate) fn run_timed(
    command: &Command,
    timeout: Duration,
) -> io::Result<(Containment, Captured)> {
    #[cfg(test)]
    RELEASES.with(|cell| cell.set(0));
    if namespace_failure_injected() {
        return Ok((
            Containment::Unavailable {
                detail: "clone3: Operation not permitted".to_string(),
            },
            empty_capture(),
        ));
    }

    let (stdout_read, stdout_write) = io::pipe()?;
    let (stderr_read, stderr_write) = io::pipe()?;
    let (mut parent_control, child_control) = control_socket()?;
    let uid = rustix::process::getuid().as_raw();
    let gid = rustix::process::getgid().as_raw();
    let image = ExecImage::prepare(
        command,
        stdout_write.as_raw_fd(),
        stderr_write.as_raw_fd(),
        child_control.as_raw_fd(),
    )?;

    let mut pidfd_slot = -1;
    let pid = match clone3(
        libc::CLONE_NEWUSER as u64 | libc::CLONE_NEWPID as u64,
        Some(&mut pidfd_slot),
        None,
    ) {
        Ok(0) => enter_init(&image),
        Ok(pid) => pid,
        Err(err) => {
            return Ok((
                Containment::Unavailable {
                    detail: format!("clone3: {err}"),
                },
                empty_capture(),
            ));
        }
    };
    drop(stdout_write);
    drop(stderr_write);
    drop(child_control);

    if pidfd_slot < 0 {
        let detail = match kill_live_pid(pid) {
            Ok(()) => "clone3 did not return a pidfd".to_string(),
            Err(err) => format!("clone3 did not return a pidfd; {err}"),
        };
        return Ok((Containment::Unkillable { detail }, empty_capture()));
    }
    // Safety: clone3 wrote a new CLOEXEC pidfd into `pidfd_slot` and this
    // process owns that descriptor.
    let pidfd = unsafe { OwnedFd::from_raw_fd(pidfd_slot) };
    let mut init = HeldInit {
        pidfd,
        reaped: false,
    };
    let deadline = Instant::now() + timeout;
    let reader_deadline = deadline + Duration::from_secs(3);
    let stdout_thread = spawn_reader(stdout_read, reader_deadline);
    let stderr_thread = spawn_reader(stderr_read, reader_deadline);
    let outcome = supervise(&mut init, pid, &mut parent_control, uid, gid, deadline);
    let stdout = join_reader(stdout_thread)?;
    let stderr = join_reader(stderr_thread)?;
    Ok((
        outcome,
        Captured {
            stdout: stdout.bytes,
            stderr: stderr.bytes,
        },
    ))
}

fn namespace_failure_injected() -> bool {
    #[cfg(test)]
    {
        FAIL_NAMESPACE.with(Cell::get)
    }
    #[cfg(not(test))]
    {
        false
    }
}

fn empty_capture() -> Captured {
    Captured {
        stdout: Vec::new(),
        stderr: Vec::new(),
    }
}

struct HeldInit {
    pidfd: OwnedFd,
    reaped: bool,
}

impl Drop for HeldInit {
    fn drop(&mut self) {
        if self.reaped {
            return;
        }
        self.reaped = true;
        if let Err(err) = kill_and_reap(&self.pidfd) {
            eprintln!("bash containment cleanup: {err}");
        }
    }
}

fn supervise(
    init: &mut HeldInit,
    pid: i32,
    control: &mut (impl Read + io::Write + AsFd),
    uid: u32,
    gid: u32,
    deadline: Instant,
) -> Containment {
    match wait_for_ack(control, &init.pidfd, deadline) {
        Ack::Ready => {}
        Ack::Exited => {
            return match reap_exit(&init.pidfd) {
                Ok(code) => {
                    init.reaped = true;
                    Containment::Unavailable {
                        detail: format!(
                            "pid namespace init did not arm parent-death signal (exit {code})"
                        ),
                    }
                }
                Err(detail) => {
                    let extra = take_down(init).err();
                    Containment::Failed(join_detail(&detail, extra.as_deref()))
                }
            };
        }
        Ack::Rejected => {
            return refuse_after_kill(init, "pid namespace init sent a bad ack");
        }
        Ack::Deadline => return finish_deadline(init),
        Ack::Failed(detail) => {
            let extra = take_down(init).err();
            return Containment::Failed(join_detail(&detail, extra.as_deref()));
        }
    }

    if let Err(err) = write_id_maps(pid, uid, gid) {
        return refuse_after_kill(init, &err.to_string());
    }
    // Maps can outrun a short timeout. Do not release a command whose deadline
    // has already passed.
    if Instant::now() >= deadline {
        return finish_deadline(init);
    }
    if let Err(err) = control.write_all(&[RELEASED]) {
        let detail = format!("release: {err}");
        let extra = take_down(init).err();
        return Containment::Unavailable {
            detail: join_detail(&detail, extra.as_deref()),
        };
    }
    #[cfg(test)]
    RELEASES.with(|cell| cell.set(cell.get().saturating_add(1)));

    match watch_init(&init.pidfd, deadline) {
        Ok(code) => {
            init.reaped = true;
            Containment::Finished(code)
        }
        Err(WatchError::Deadline) => finish_deadline(init),
        Err(WatchError::Failed(detail)) => {
            let extra = take_down(init).err();
            Containment::Failed(join_detail(&detail, extra.as_deref()))
        }
    }
}

fn refuse_after_kill(init: &mut HeldInit, detail: &str) -> Containment {
    match take_down(init) {
        Ok(()) => Containment::Unavailable {
            detail: detail.to_string(),
        },
        Err(err) => Containment::Unkillable {
            detail: join_detail(detail, Some(&err)),
        },
    }
}

fn finish_deadline(init: &mut HeldInit) -> Containment {
    match take_down(init) {
        Ok(()) => Containment::TimedOut,
        Err(detail) => Containment::Unkillable { detail },
    }
}

fn take_down(init: &mut HeldInit) -> Result<(), String> {
    if init.reaped {
        return Ok(());
    }
    init.reaped = true;
    kill_and_reap(&init.pidfd)
}

enum Ack {
    Ready,
    Exited,
    Rejected,
    Deadline,
    Failed(String),
}

fn wait_for_ack(ack: &mut (impl Read + AsFd), pidfd: &OwnedFd, deadline: Instant) -> Ack {
    loop {
        if Instant::now() >= deadline {
            return Ack::Deadline;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        let timeout = Timespec::try_from(remaining).ok();
        let mut fds = [
            PollFd::new(ack, PollFlags::IN),
            PollFd::new(pidfd, PollFlags::IN),
        ];
        match rustix::event::poll(&mut fds, timeout.as_ref()) {
            Ok(0) => return Ack::Deadline,
            Ok(_) => {
                let ack_ready = fds[0]
                    .revents()
                    .intersects(PollFlags::IN | PollFlags::HUP | PollFlags::ERR);
                let init_ready = fds[1]
                    .revents()
                    .intersects(PollFlags::IN | PollFlags::HUP | PollFlags::ERR);
                if ack_ready {
                    match read_ack(ack) {
                        Ok(Some(RELEASED)) => return Ack::Ready,
                        Ok(Some(_)) => return Ack::Rejected,
                        Ok(None) => return Ack::Exited,
                        Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                        Err(err) => return Ack::Failed(format!("ack: {err}")),
                    }
                } else if init_ready {
                    return Ack::Exited;
                }
            }
            Err(Errno::INTR) => {}
            Err(err) => return Ack::Failed(format!("ack wait: {err}")),
        }
    }
}

fn read_ack(ack: &mut impl Read) -> io::Result<Option<u8>> {
    let mut buf = [0_u8; 1];
    match ack.read(&mut buf)? {
        0 => Ok(None),
        _ => Ok(Some(buf[0])),
    }
}

enum WatchError {
    Deadline,
    Failed(String),
}

fn watch_init(pidfd: &OwnedFd, deadline: Instant) -> Result<i32, WatchError> {
    loop {
        let deadline_reached = Instant::now() >= deadline;
        let remaining = deadline.saturating_duration_since(Instant::now());
        let timeout = Timespec::try_from(remaining).ok();
        let mut fds = [PollFd::new(pidfd, PollFlags::IN)];
        let poll = if deadline_reached {
            rustix::event::poll(&mut fds, Some(&Timespec::default()))
        } else {
            rustix::event::poll(&mut fds, timeout.as_ref())
        };
        let ready = match poll {
            Ok(0) => false,
            Ok(_) => fds[0]
                .revents()
                .intersects(PollFlags::IN | PollFlags::HUP | PollFlags::ERR),
            Err(Errno::INTR) => continue,
            Err(err) => {
                return Err(WatchError::Failed(format!("pidfd wait: {err}")));
            }
        };
        if !ready {
            if deadline_reached {
                return Err(WatchError::Deadline);
            }
            continue;
        }
        match reap_once(pidfd) {
            Reap::Exited(code) => return Ok(code),
            Reap::Gone => {
                return Err(WatchError::Failed(
                    "wait: pid namespace init was not a child".to_string(),
                ));
            }
            Reap::Retry if deadline_reached => return Err(WatchError::Deadline),
            Reap::Retry => thread::sleep(Duration::from_millis(1)),
            Reap::Failed(detail) => return Err(WatchError::Failed(detail)),
        }
    }
}

#[derive(Debug)]
enum Reap {
    Exited(i32),
    Retry,
    Gone,
    Failed(String),
}

/// A `waitid` error is failed immediately. `Ok(None)` only means the child is
/// still running, so the caller may wait out the deadline.
fn classify_reap(result: Result<Option<i32>, String>) -> Reap {
    match result {
        Err(detail) => Reap::Failed(detail),
        Ok(Some(code)) => Reap::Exited(code),
        Ok(None) => Reap::Retry,
    }
}

fn reap_once(pidfd: &OwnedFd) -> Reap {
    let waited = match rustix::process::waitid(
        WaitId::PidFd(pidfd.as_fd()),
        WaitIdOptions::EXITED.union(WaitIdOptions::NOHANG),
    ) {
        Ok(Some(status)) => Ok(Some(exit_code_from_waitid(&status))),
        Ok(None) | Err(Errno::INTR) => Ok(None),
        Err(Errno::CHILD) => return Reap::Gone,
        Err(err) => Err(format!("wait: {err}")),
    };
    classify_reap(waited)
}

fn reap_exit(pidfd: &OwnedFd) -> Result<i32, String> {
    let deadline = Instant::now() + EMPTY_WAIT;
    loop {
        match reap_once(pidfd) {
            Reap::Exited(code) => return Ok(code),
            Reap::Gone => return Err("wait: pid namespace init was not a child".to_string()),
            Reap::Failed(detail) => return Err(detail),
            Reap::Retry if Instant::now() >= deadline => {
                return Err("wait returned no status".to_string());
            }
            Reap::Retry => thread::sleep(Duration::from_millis(10)),
        }
    }
}

fn kill_and_reap(pidfd: &OwnedFd) -> Result<(), String> {
    let killed = match signal_pidfd(pidfd) {
        Ok(()) => Ok(()),
        Err(err) => Err(format!("pidfd kill: {err}")),
    };
    let waited = wait_until_dead(pidfd);
    match (killed, waited) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(kill_error), Ok(())) => Err(kill_error),
        (Ok(()), Err(wait_error)) => Err(wait_error),
        (Err(kill_error), Err(wait_error)) => Err(format!("{kill_error}; {wait_error}")),
    }
}

fn signal_pidfd(pidfd: &OwnedFd) -> Result<(), Errno> {
    match rustix::process::pidfd_send_signal(pidfd, Signal::KILL) {
        Ok(()) | Err(Errno::SRCH) => Ok(()),
        Err(err) => Err(err),
    }
}

fn wait_until_dead(pidfd: &OwnedFd) -> Result<(), String> {
    let deadline = Instant::now() + EMPTY_WAIT;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let timeout = Timespec::try_from(remaining).ok();
        let mut fds = [PollFd::new(pidfd, PollFlags::IN)];
        match rustix::event::poll(&mut fds, timeout.as_ref()) {
            Ok(0) => return Err("process still running after SIGKILL".to_string()),
            Ok(_) => {
                return match reap_once(pidfd) {
                    Reap::Exited(_) | Reap::Gone => Ok(()),
                    Reap::Failed(detail) => Err(detail),
                    Reap::Retry if Instant::now() >= deadline => {
                        Err("process still running after SIGKILL".to_string())
                    }
                    Reap::Retry => continue,
                };
            }
            Err(Errno::INTR) if Instant::now() >= deadline => {
                return Err("process still running after SIGKILL".to_string());
            }
            Err(Errno::INTR) => {}
            Err(err) => return Err(format!("pidfd wait: {err}")),
        }
    }
}

fn kill_live_pid(pid: i32) -> Result<(), String> {
    let Some(pid) = rustix::process::Pid::from_raw(pid) else {
        return Err("kill: pid is not a process id".to_string());
    };
    match rustix::process::kill_process(pid, Signal::KILL) {
        Ok(()) | Err(Errno::SRCH) => {}
        Err(err) => return Err(format!("kill: {err}")),
    }
    match rustix::process::waitpid(Some(pid), WaitOptions::empty()) {
        Ok(_) | Err(Errno::CHILD) => Ok(()),
        Err(err) => Err(format!("wait: {err}")),
    }
}

fn write_id_maps(pid: i32, uid: u32, gid: u32) -> io::Result<()> {
    let proc = format!("/proc/{pid}");
    std::fs::write(format!("{proc}/setgroups"), b"deny\n")
        .map_err(|err| io::Error::other(format!("setgroups: {err}")))?;
    std::fs::write(format!("{proc}/uid_map"), format!("0 {uid} 1\n"))
        .map_err(|err| io::Error::other(format!("uid_map: {err}")))?;
    std::fs::write(format!("{proc}/gid_map"), format!("0 {gid} 1\n"))
        .map_err(|err| io::Error::other(format!("gid_map: {err}")))?;
    Ok(())
}

fn exit_code_from_waitid(status: &WaitIdStatus) -> i32 {
    if let Some(code) = status.exit_status() {
        return code;
    }
    status
        .terminating_signal()
        .map_or(1, |signal| 128_i32.saturating_add(signal))
}

fn exit_code_from_wait(status: WaitStatus) -> i32 {
    if let Some(code) = status.exit_status() {
        return code;
    }
    status
        .terminating_signal()
        .map_or(1, |signal| 128_i32.saturating_add(signal))
}

fn join_detail(left: &str, right: Option<&str>) -> String {
    match right {
        None | Some("") => left.to_string(),
        Some(right) if left.is_empty() => right.to_string(),
        Some(right) => format!("{left}; {right}"),
    }
}

struct PipeRead {
    bytes: Vec<u8>,
}

fn spawn_reader(
    pipe: impl Read + AsFd + Send + 'static,
    deadline: Instant,
) -> JoinHandle<io::Result<PipeRead>> {
    thread::spawn(move || read_pipe_until(pipe, deadline))
}

fn read_pipe_until(mut pipe: impl Read + AsFd, deadline: Instant) -> io::Result<PipeRead> {
    let mut bytes = Vec::new();
    let mut buf = [0_u8; 8_192];
    loop {
        if Instant::now() >= deadline {
            return Ok(PipeRead { bytes });
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        let Ok(timeout) = Timespec::try_from(remaining) else {
            return Ok(PipeRead { bytes });
        };
        let events = {
            let mut fds = [PollFd::new(&pipe, PollFlags::IN)];
            match rustix::event::poll(&mut fds, Some(&timeout)) {
                Ok(0) => return Ok(PipeRead { bytes }),
                Ok(_) => fds[0].revents(),
                Err(Errno::INTR) => continue,
                Err(err) => return Err(err.into()),
            }
        };
        if events.contains(PollFlags::NVAL) {
            return Err(io::Error::other("command output pipe is not pollable"));
        }
        if !events.intersects(PollFlags::IN | PollFlags::HUP | PollFlags::ERR) {
            continue;
        }
        match pipe.read(&mut buf) {
            Ok(0) => return Ok(PipeRead { bytes }),
            Ok(count) => bytes.extend_from_slice(&buf[..count]),
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                ) => {}
            Err(err) => return Err(err),
        }
    }
}

fn join_reader(handle: JoinHandle<io::Result<PipeRead>>) -> io::Result<PipeRead> {
    match handle.join() {
        Ok(result) => result,
        Err(_) => Err(io::Error::other(
            "command output reader stopped unexpectedly",
        )),
    }
}

struct ExecImage {
    program: CString,
    argv: Vec<CString>,
    envp: Vec<CString>,
    cwd: Option<CString>,
    argv_ptrs: Vec<usize>,
    env_ptrs: Vec<usize>,
    stdout_write: i32,
    stderr_write: i32,
    control: i32,
    original_parent: i32,
    fail_arming: bool,
}

impl ExecImage {
    fn prepare(
        command: &Command,
        stdout_write: i32,
        stderr_write: i32,
        control: i32,
    ) -> io::Result<Self> {
        let program = resolve_program(command.get_program())?;
        let mut argv = Vec::new();
        argv.push(cstring(command.get_program())?);
        for arg in command.get_args() {
            argv.push(cstring(arg)?);
        }
        let envp = environment_from(command)?;
        let cwd = command
            .get_current_dir()
            .map(|dir| cstring(dir.as_os_str()))
            .transpose()?;
        let argv_ptrs = pointer_words(&argv);
        let env_ptrs = pointer_words(&envp);
        Ok(Self {
            program,
            argv,
            envp,
            cwd,
            argv_ptrs,
            env_ptrs,
            stdout_write,
            stderr_write,
            control,
            original_parent: rustix::process::getpid().as_raw_pid(),
            fail_arming: arming_failure_injected(),
        })
    }
}

fn arming_failure_injected() -> bool {
    #[cfg(test)]
    {
        FAIL_ARMING.with(Cell::get)
    }
    #[cfg(not(test))]
    {
        false
    }
}

fn resolve_program(program: &OsStr) -> io::Result<CString> {
    let search = std::env::var_os("PATH").unwrap_or_else(|| "/bin:/usr/bin".into());
    resolve_program_in(program, search.as_os_str())
}

fn resolve_program_in(program: &OsStr, search: &OsStr) -> io::Result<CString> {
    if program.as_bytes().contains(&b'/') {
        return cstring(program);
    }
    for dir in std::env::split_paths(search) {
        let candidate = dir.join(program);
        if !candidate.is_file() {
            continue;
        }
        if rustix::fs::access(&candidate, rustix::fs::Access::EXEC_OK).is_err() {
            continue;
        }
        return cstring(candidate.as_os_str());
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("command not found: {}", program.to_string_lossy()),
    ))
}

fn cstring(value: &OsStr) -> io::Result<CString> {
    CString::new(value.as_bytes()).map_err(|_| io::Error::other("command argument contains a NUL"))
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

fn pointer_words(values: &[CString]) -> Vec<usize> {
    let mut words = values
        .iter()
        .map(|value| value.as_ptr() as usize)
        .collect::<Vec<_>>();
    words.push(0);
    words
}

fn control_socket() -> io::Result<(UnixStream, UnixStream)> {
    let mut fds = [-1_i32; 2];
    // Safety: `socketpair` writes two new descriptors into `fds` on success.
    let rc = unsafe {
        libc::socketpair(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_CLOEXEC,
            0,
            fds.as_mut_ptr(),
        )
    };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    // Safety: both descriptors were created by `socketpair` and this process
    // owns them.
    let parent = unsafe { UnixStream::from_raw_fd(fds[0]) };
    let child = unsafe { UnixStream::from_raw_fd(fds[1]) };
    Ok((parent, child))
}

fn clone3(flags: u64, pidfd: Option<&mut i32>, set_tid: Option<&mut i32>) -> io::Result<i32> {
    clone3_raw(flags, pidfd, set_tid).map_err(io::Error::from_raw_os_error)
}

/// Raw `clone3`. The child calls this and `_exit`s on the errno. It does not
/// build an `io::Error`.
fn clone3_raw(flags: u64, pidfd: Option<&mut i32>, set_tid: Option<&mut i32>) -> Result<i32, i32> {
    // Safety: every field is a `u64`. Zero is the documented empty `clone_args`.
    let mut args: libc::clone_args = unsafe { std::mem::zeroed() };
    args.flags = flags;
    args.exit_signal = libc::SIGCHLD as u64;
    if let Some(slot) = pidfd {
        *slot = -1;
        args.flags |= libc::CLONE_PIDFD as u64;
        args.pidfd = std::ptr::from_mut::<i32>(slot) as u64;
    }
    if let Some(tid) = set_tid {
        args.set_tid = std::ptr::from_mut::<i32>(tid) as u64;
        args.set_tid_size = 1;
    }
    // Safety: `stack` is null and `CLONE_VM` is not set, so the child gets its
    // own address space and continues after the syscall, the same way `fork`
    // does. The child must exit instead of unwinding through this frame.
    let ret = unsafe { libc::syscall(libc::SYS_clone3, &args, std::mem::size_of_val(&args)) };
    if ret < 0 {
        Err(last_errno())
    } else {
        i32::try_from(ret).map_err(|_| libc::ERANGE)
    }
}

fn enter_init(image: &ExecImage) -> ! {
    retain_allowlisted_fds(image.control, image.stdout_write, image.stderr_write);
    if image.fail_arming {
        raw_exit(ARM_FAILED);
    }
    // Safety: `PR_SET_PDEATHSIG` takes the signal in the second argument.
    if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) } != 0 {
        raw_exit(ARM_FAILED);
    }
    // Inside the new pid namespace the creator is reported as 0. Any other
    // unexpected parent means this process was reparented before the death
    // signal was armed.
    if !parent_is_original(raw_getppid(), image.original_parent) {
        raw_exit(ARM_FAILED);
    }
    if write_full(FD_CONTROL, &[RELEASED]).is_err() {
        raw_exit(ARM_FAILED);
    }
    let mut release = [0_u8; 1];
    if read_full(FD_CONTROL, &mut release).is_err() || release[0] != RELEASED {
        raw_exit(ARM_FAILED);
    }
    // Leave the runtime's session before the command starts. `kill -STOP 0`
    // in the command must not stop the process that holds the init pidfd.
    if unsafe { libc::setsid() } < 0 {
        raw_exit(126);
    }
    match clone3_raw(0, None, None) {
        Ok(0) => become_command(image),
        Ok(shell) => {
            unsafe {
                libc::close(FD_STDOUT);
                libc::close(FD_STDERR);
            }
            reap_until_empty(shell);
        }
        Err(_) => raw_exit(127),
    }
}

/// Dup the control socket and the command's stdout and stderr onto fds 0, 1,
/// and 2, then close every inherited descriptor from 3 up.
fn retain_allowlisted_fds(control: i32, stdout: i32, stderr: i32) {
    let control = park_above_stdio(control);
    let stdout = park_above_stdio(stdout);
    let stderr = park_above_stdio(stderr);
    let moved =
        dup_onto(control, FD_CONTROL) && dup_onto(stdout, FD_STDOUT) && dup_onto(stderr, FD_STDERR);
    // Safety: fds below 3 are the allowlist installed just above. Flag 0
    // closes the range instead of only marking it cloexec.
    let closed = unsafe { libc::close_range(3, libc::c_uint::MAX, 0) };
    if !moved || closed < 0 {
        raw_exit(ARM_FAILED);
    }
}

fn park_above_stdio(fd: i32) -> i32 {
    if fd >= 3 {
        return fd;
    }
    // Safety: `F_DUPFD` copies `fd` to the lowest free number at or above 3.
    let moved = unsafe { libc::fcntl(fd, libc::F_DUPFD, 3) };
    if moved < 0 {
        return -1;
    }
    unsafe { libc::close(fd) };
    moved
}

fn raw_exit(code: i32) -> ! {
    unsafe { libc::_exit(code) }
}

fn raw_getppid() -> i32 {
    unsafe { libc::getppid() }
}

fn parent_is_original(ppid: i32, original: i32) -> bool {
    ppid == original || (ppid == 0 && original > 0)
}

fn become_command(image: &ExecImage) -> ! {
    if image.argv.len() + 1 != image.argv_ptrs.len() || image.envp.len() + 1 != image.env_ptrs.len()
    {
        raw_exit(127);
    }
    if let Some(cwd) = &image.cwd {
        // Safety: `cwd` is a NUL-terminated path prepared before `clone3`.
        if unsafe { libc::chdir(cwd.as_ptr()) } < 0 {
            raw_exit(127);
        }
    }
    // Safety: fds 1 and 2 are the command pipes. `/dev/null` replaces the
    // control socket on fd 0. The pointer tables stay valid until `execve`.
    unsafe {
        let null = libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY);
        if null < 0 || !dup_onto(null, FD_CONTROL) {
            raw_exit(127);
        }
        if null != FD_CONTROL {
            libc::close(null);
        }
        if !dup_onto(FD_STDOUT, FD_STDOUT) || !dup_onto(FD_STDERR, FD_STDERR) {
            raw_exit(127);
        }
        libc::execve(
            image.program.as_ptr(),
            image.argv_ptrs.as_ptr().cast(),
            image.env_ptrs.as_ptr().cast(),
        );
    }
    raw_exit(127);
}

/// `dup2(fd, fd)` does not clear `FD_CLOEXEC`. Clear it explicitly when the
/// source is already the target so the executed command keeps stdout and stderr.
fn dup_onto(src: i32, dst: i32) -> bool {
    if src < 0 {
        return false;
    }
    unsafe {
        if src == dst {
            let flags = libc::fcntl(dst, libc::F_GETFD);
            if flags < 0 {
                return false;
            }
            return libc::fcntl(dst, libc::F_SETFD, flags & !libc::FD_CLOEXEC) >= 0;
        }
        libc::dup2(src, dst) >= 0
    }
}

fn reap_until_empty(shell: i32) -> ! {
    let mut code = 127;
    let mut latched = false;
    loop {
        match rustix::process::wait(WaitOptions::empty()) {
            Ok(Some((pid, status))) => {
                // A later `clone3(set_tid = shell)` reuses the pid. Record the
                // shell status once and keep reaping without adopting the
                // impostor's code.
                if !latched && pid.as_raw_pid() == shell {
                    code = exit_code_from_wait(status);
                    latched = true;
                }
            }
            Ok(None) | Err(Errno::INTR) => {}
            Err(Errno::CHILD) => raw_exit(code),
            Err(_) => raw_exit(126),
        }
    }
}

fn last_errno() -> i32 {
    unsafe { *libc::__errno_location() }
}

fn write_full(fd: i32, mut buf: &[u8]) -> Result<(), i32> {
    while !buf.is_empty() {
        // Safety: `fd` is open in this process for the duration of the write.
        let count = unsafe { libc::write(fd, buf.as_ptr().cast(), buf.len()) };
        if count < 0 {
            let err = last_errno();
            if err == libc::EINTR {
                continue;
            }
            return Err(err);
        }
        if count == 0 {
            return Err(libc::EIO);
        }
        let count = usize::try_from(count).map_err(|_| libc::EIO)?;
        buf = &buf[count..];
    }
    Ok(())
}

fn read_full(fd: i32, buf: &mut [u8]) -> Result<(), i32> {
    let mut filled = 0;
    while filled < buf.len() {
        // Safety: `fd` is open in this process for the duration of the read.
        let count =
            unsafe { libc::read(fd, buf[filled..].as_mut_ptr().cast(), buf.len() - filled) };
        if count < 0 {
            let err = last_errno();
            if err == libc::EINTR {
                continue;
            }
            return Err(err);
        }
        if count == 0 {
            return Err(libc::EPIPE);
        }
        filled += usize::try_from(count).map_err(|_| libc::EIO)?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::{
        classify_reap, clone3, clone3_raw, exit_code_from_waitid, parent_is_original,
        reap_until_empty, resolve_program_in, signal_pidfd, write_full, write_id_maps, Reap,
    };
    use std::ffi::OsStr;
    use std::io::Read;
    use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn wait_error_is_not_a_timeout_even_at_the_deadline() {
        let outcome = classify_reap(Err("wait: bad file descriptor".to_string()));
        assert!(
            matches!(outcome, Reap::Failed(ref detail) if detail.contains("wait:")),
            "a wait error must not lapse into a timeout: {outcome:?}"
        );
        assert!(matches!(classify_reap(Ok(None)), Reap::Retry));
        assert!(matches!(classify_reap(Ok(Some(0))), Reap::Exited(0)));
    }

    #[test]
    fn pid_namespace_init_sees_its_outside_parent_as_zero() {
        assert!(parent_is_original(0, 42));
        assert!(parent_is_original(42, 42));
        assert!(!parent_is_original(1, 42));
        assert!(!parent_is_original(7, 42));
    }

    #[test]
    fn path_lookup_skips_a_non_executable_file() {
        let root = std::env::temp_dir().join(format!("claw-path-{}", std::process::id()));
        let first = root.join("first");
        let second = root.join("second");
        std::fs::create_dir_all(&first).expect("first PATH dir");
        std::fs::create_dir_all(&second).expect("second PATH dir");
        let blocked = first.join("claw-sample");
        let real = second.join("claw-sample");
        std::fs::write(&blocked, b"#!/bin/sh\nprintf blocked\n").expect("blocked file");
        std::fs::write(&real, b"#!/bin/sh\nprintf ok\n").expect("executable file");
        std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o644))
            .expect("mode 0644");
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o755)).expect("mode 0755");
        let search = std::env::join_paths([&first, &second]).expect("PATH");
        let found = resolve_program_in(OsStr::new("claw-sample"), search.as_os_str())
            .expect("later executable");
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(found.as_bytes(), real.as_os_str().as_bytes());
    }

    #[test]
    fn reaped_init_pidfd_does_not_kill_a_process_that_reused_its_pid() {
        let (result_read, result_write) = std::io::pipe().expect("result pipe");
        let (ack_read, ack_write) = std::io::pipe().expect("ack pipe");
        let (release_read, release_write) = std::io::pipe().expect("release pipe");
        let uid = rustix::process::getuid().as_raw();
        let gid = rustix::process::getgid().as_raw();
        let mut pidfd_slot = -1;
        let pid = clone3(
            libc::CLONE_NEWUSER as u64 | libc::CLONE_NEWPID as u64,
            Some(&mut pidfd_slot),
            None,
        )
        .expect("lab namespace");
        if pid == 0 {
            // Close the parent's ends in this child only. The `OwnedFd` values
            // stay with the parent; `close` affects this fd table.
            for fd in [
                result_read.as_raw_fd(),
                ack_read.as_raw_fd(),
                release_write.as_raw_fd(),
            ] {
                unsafe { libc::close(fd) };
            }
            lab(
                ack_write.as_raw_fd(),
                release_read.as_raw_fd(),
                result_write.as_raw_fd(),
            );
        }
        drop(ack_write);
        drop(release_read);
        drop(result_write);
        assert!(pidfd_slot >= 0, "lab clone3 did not return a pidfd");
        // Safety: clone3 installed this pidfd in the parent.
        let pidfd = unsafe { OwnedFd::from_raw_fd(pidfd_slot) };
        let mut ack = [0_u8; 1];
        ack_read.take(1).read_exact(&mut ack).expect("lab ack");
        write_id_maps(pid, uid, gid).expect("lab id maps");
        let mut release = release_write;
        std::io::Write::write_all(&mut release, &[1]).expect("lab release");
        drop(release);

        let mut message = String::new();
        result_read
            .take(64)
            .read_to_string(&mut message)
            .expect("lab result");
        let _ = rustix::process::waitpid(
            rustix::process::Pid::from_raw(pid),
            rustix::process::WaitOptions::empty(),
        );
        drop(pidfd);
        assert_eq!(
            message, "Y",
            "signaling the reaped init pidfd affected the process that reused its pid ({message})"
        );
    }

    #[test]
    fn shell_status_stays_latched_when_its_pid_is_reused() {
        const SHELL_STATUS: i32 = 42;
        let (mut result_read, result_write) = std::io::pipe().expect("result pipe");
        let (ack_read, ack_write) = std::io::pipe().expect("ack pipe");
        let (release_read, release_write) = std::io::pipe().expect("release pipe");
        let uid = rustix::process::getuid().as_raw();
        let gid = rustix::process::getgid().as_raw();
        let mut pidfd_slot = -1;
        let pid = clone3(
            libc::CLONE_NEWUSER as u64 | libc::CLONE_NEWPID as u64,
            Some(&mut pidfd_slot),
            None,
        )
        .expect("lab namespace");
        if pid == 0 {
            for fd in [
                result_read.as_raw_fd(),
                ack_read.as_raw_fd(),
                release_write.as_raw_fd(),
            ] {
                unsafe { libc::close(fd) };
            }
            latch_lab(
                ack_write.as_raw_fd(),
                release_read.as_raw_fd(),
                result_write.as_raw_fd(),
            );
        }
        drop(ack_write);
        drop(release_read);
        drop(result_write);
        assert!(pidfd_slot >= 0, "lab clone3 did not return a pidfd");
        let pidfd = unsafe { OwnedFd::from_raw_fd(pidfd_slot) };
        let mut ack = [0_u8; 1];
        ack_read.take(1).read_exact(&mut ack).expect("lab ack");
        write_id_maps(pid, uid, gid).expect("lab id maps");
        let mut release = release_write;
        std::io::Write::write_all(&mut release, &[1]).expect("lab release");
        drop(release);

        let mut marker = [0_u8; 2];
        let n = result_read.read(&mut marker).expect("reuse marker");
        let marker = &marker[..n];
        let status = rustix::process::waitid(
            rustix::process::WaitId::PidFd(pidfd.as_fd()),
            rustix::process::WaitIdOptions::EXITED,
        )
        .expect("reap lab")
        .expect("lab status");
        assert_eq!(
            marker, b"Y",
            "the production reaper never saw a reused shell pid ({marker:?})"
        );
        assert_eq!(
            exit_code_from_waitid(&status),
            SHELL_STATUS,
            "a descendant that reused the shell pid overwrote its exit status"
        );
    }

    fn latch_lab(ack: i32, release: i32, result: i32) -> ! {
        if write_full(ack, &[1]).is_err() {
            super::raw_exit(2);
        }
        let mut byte = [0_u8; 1];
        if super::read_full(release, &mut byte).is_err() || byte[0] != 1 {
            super::raw_exit(2);
        }
        let shell = match clone3_raw(0, None, None) {
            Ok(0) => super::raw_exit(42),
            Ok(pid) => pid,
            Err(_) => {
                let _ = write_full(result, b"S");
                super::raw_exit(70);
            }
        };
        match clone3_raw(0, None, None) {
            Ok(0) => reuse_shell_pid(shell, result),
            Ok(_) => {}
            Err(_) => {
                let _ = write_full(result, b"I");
                super::raw_exit(71);
            }
        }
        reap_until_empty(shell);
    }

    fn reuse_shell_pid(shell: i32, result: i32) -> ! {
        let mut tid = shell;
        for _ in 0..1_000_000 {
            match clone3_raw(0, None, Some(&mut tid)) {
                Ok(0) => super::raw_exit(99),
                Ok(pid) => {
                    let marker: &[u8] = if pid == shell { b"Y" } else { b"M" };
                    let _ = write_full(result, marker);
                    // Do not reap. The child is reparented to the namespace
                    // init, so the production reaper collects its status.
                    super::raw_exit(0);
                }
                Err(err) if err == libc::EEXIST => {}
                Err(err) => {
                    let marker = [b'E', u8::try_from(err).unwrap_or(255)];
                    let _ = write_full(result, &marker);
                    super::raw_exit(86);
                }
            }
        }
        let _ = write_full(result, b"T");
        super::raw_exit(86);
    }

    fn lab(ack: i32, release: i32, result: i32) -> ! {
        if write_full(ack, &[1]).is_err() {
            super::raw_exit(2);
        }
        let mut byte = [0_u8; 1];
        if super::read_full(release, &mut byte).is_err() || byte[0] != 1 {
            super::raw_exit(2);
        }
        let mut first_slot = -1;
        let first = match clone3(0, Some(&mut first_slot), None) {
            Ok(0) => super::raw_exit(0),
            Ok(pid) => pid,
            Err(_) => report(result, b'E'),
        };
        // Safety: clone3 installed this pidfd for the first child.
        let first_fd = unsafe { OwnedFd::from_raw_fd(first_slot) };
        unsafe {
            let _ = libc::waitpid(first, std::ptr::null_mut(), 0);
        }
        let mut wanted = first;
        let mut second_slot = -1;
        let second = match clone3(0, Some(&mut second_slot), Some(&mut wanted)) {
            Ok(0) => {
                unsafe {
                    let _ = libc::pause();
                }
                super::raw_exit(0);
            }
            Ok(pid) => pid,
            Err(_) => report(result, b'E'),
        };
        // Safety: clone3 installed this pidfd for the reused pid.
        let second_fd = unsafe { OwnedFd::from_raw_fd(second_slot) };
        let signaled = signal_pidfd(&first_fd);
        let alive = pidfd_alive(&second_fd);
        let message: &[u8] = if signaled.is_err() {
            b"S"
        } else if second != first {
            b"M"
        } else if alive {
            b"Y"
        } else {
            b"N"
        };
        let _ = write_full(result, message);
        let _ = signal_pidfd(&second_fd);
        super::raw_exit(0);
    }

    fn report(result: i32, kind: u8) -> ! {
        let _ = write_full(result, &[kind]);
        super::raw_exit(1);
    }

    fn pidfd_alive(pidfd: &OwnedFd) -> bool {
        // Safety: signal 0 on a pidfd checks that the same task is alive. It
        // does not follow a reused pid number.
        let rc = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                pidfd.as_raw_fd(),
                0,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
        rc == 0
    }
}
