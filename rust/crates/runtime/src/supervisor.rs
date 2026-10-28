//! Contain one timed shell command, then tear it down.
//!
//! The supervisor is its own session. It starts the command in a cgroup v2
//! domain created before exec (`<domain>/cmd`, with an empty `<domain>/supervisor`
//! sibling). Emptiness is `cgroup.events` `populated 0`, watched with `poll`.
//! Teardown repeats `cgroup.kill` until that field is 0, then removes the
//! directory. A process that has moved to an ancestor is killed by pidfd; if
//! it is still alive after the wait the result is `unkillable`, never exit 127.
//!
//! When the domain cannot be created or the command cannot join it, the
//! command runs in a pid namespace. Teardown kills that namespace's init and
//! waits until the init pidfd reports the process is gone.

use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::fd::OwnedFd;
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec};

/// How long teardown waits for the cgroup or pid namespace to become empty.
const EMPTY_WAIT: Duration = Duration::from_secs(2);

/// Status text for a process that survived teardown. Not an exit code.
pub(crate) const UNKILLABLE_STATUS: &str = "processes could not be killed";

const CGROUP_PREFIX: &str = "bash-supervisor-";

pub(crate) enum SupervisorOutcome {
    Finished(i32),
    Timeout,
    Unkillable { detail: String },
    Unavailable { detail: String },
    Failed(String),
}

/// What the parent has learned from the status file so far.
///
/// A failed cgroup join falls back to a pid namespace, so both may be set.
/// The parent kills the cgroup and the namespace init if the supervisor exits.
#[derive(Debug, Default)]
pub(crate) struct LiveContainment {
    pub cgroup: Option<PathBuf>,
    pub shell: Option<u32>,
    pub unshare: Option<u32>,
    pub init: Option<u32>,
}

pub(crate) struct CgroupCleanup {
    pub empty: bool,
    pub detail: String,
}

pub(crate) fn bash_supervisor_main() -> i32 {
    match supervise() {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

fn supervise() -> io::Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "--pid1") {
        let status = args
            .get(1)
            .ok_or_else(|| io::Error::other("pid namespace supervisor requires a status file"))?;
        let command = args
            .get(2)
            .ok_or_else(|| io::Error::other("pid namespace supervisor requires a command"))?;
        return run_pid1(Path::new(status), command);
    }

    let (status_path, timeout_ms, command) = parse_outer(&args)?;
    let mut status = StatusFile::open(&status_path)?;
    match prepare_session() {
        SessionStart::Ready => {}
        SessionStart::ParentExited => {
            status.line("mode none")?;
            status.line("result unavailable parent exited before the supervisor started")?;
            return Ok(());
        }
        SessionStart::Failed(err) => {
            status.line("mode none")?;
            status.line(&format!("result error {}", one_line(&err)))?;
            return Ok(());
        }
    }

    let deadline = Instant::now() + duration_from_millis(timeout_ms);
    match ShellCgroup::create() {
        Ok(cgroup) => {
            status.line(&format!("mode cgroup {}", cgroup.domain.display()))?;
            match spawn_shell(&command, &cgroup) {
                Ok(shell) => {
                    status.line(&format!("shell {}", shell.id()))?;
                    supervise_cgroup(&mut status, &cgroup, shell, deadline)
                }
                Err(err) => {
                    // Join failed before exec, so the command has not started.
                    fallback_pid_namespace(
                        &mut status,
                        &status_path,
                        &command,
                        deadline,
                        &one_line(&err),
                    )
                }
            }
        }
        Err(cgroup_err) => fallback_pid_namespace(
            &mut status,
            &status_path,
            &command,
            deadline,
            &one_line(&cgroup_err),
        ),
    }
}

fn fallback_pid_namespace(
    status: &mut StatusFile,
    status_path: &Path,
    command: &str,
    deadline: Instant,
    cgroup_error: &str,
) -> io::Result<()> {
    match spawn_pid_namespace(status_path, command) {
        Ok(unshare) => {
            status.line(&format!("mode pidns {}", unshare.id()))?;
            supervise_pid_namespace(status, unshare, status_path, deadline, cgroup_error)
        }
        Err(pidns_err) => {
            status.line("mode none")?;
            status.line(&format!(
                "result unavailable cgroup: {cgroup_error}; pidns: {}",
                one_line(&pidns_err)
            ))?;
            Ok(())
        }
    }
}

fn parse_outer(args: &[String]) -> io::Result<(PathBuf, u64, String)> {
    let [status, timeout, command] = args else {
        return Err(io::Error::other(
            "bash supervisor requires a status file, timeout, and command",
        ));
    };
    let timeout_ms = timeout.parse::<u64>().map_err(|_| {
        io::Error::other("bash supervisor timeout must be a number of milliseconds")
    })?;
    Ok((PathBuf::from(status), timeout_ms, command.clone()))
}

enum SessionStart {
    Ready,
    ParentExited,
    Failed(io::Error),
}

fn prepare_session() -> SessionStart {
    if let Err(err) = rustix::process::setsid() {
        return SessionStart::Failed(io::Error::other(format!("setsid: {err}")));
    }
    if let Err(err) =
        rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL))
    {
        return SessionStart::Failed(io::Error::other(format!("parent-death signal: {err}")));
    }
    // `getppid` is `None` for pid 0. Inside a pid namespace that is pid 1's
    // parent, so this check stays in the outer supervisor only.
    if rustix::process::getppid() == Some(rustix::process::Pid::INIT) {
        return SessionStart::ParentExited;
    }
    SessionStart::Ready
}

fn spawn_shell(command: &str, cgroup: &ShellCgroup) -> io::Result<Child> {
    let mut shell = Command::new("/bin/sh");
    shell.arg("-c").arg(command).stdin(Stdio::null());
    crate::contain::isolate_shell(&mut shell, &cgroup.cmd_procs);
    shell.spawn()
}

fn spawn_pid_namespace(status: &Path, command: &str) -> io::Result<Child> {
    // `/proc/self/exe` would be resolved by `unshare` and point at unshare.
    let executable = fs::read_link("/proc/self/exe")?;
    let mut unshare = Command::new("/usr/bin/unshare");
    unshare
        .args([
            "--user",
            "--map-root-user",
            "--pid",
            "--fork",
            "--mount-proc",
            "--kill-child",
            "--",
        ])
        .arg(executable)
        .arg("--pid1")
        .arg(status)
        .arg(command)
        .stdin(Stdio::null());
    crate::contain::parent_death_child(&mut unshare);
    unshare.spawn()
}

fn supervise_cgroup(
    status: &mut StatusFile,
    cgroup: &ShellCgroup,
    mut shell: Child,
    deadline: Instant,
) -> io::Result<()> {
    let mut shell_status = None;
    let shell_pidfd = open_pidfd(shell.id());
    let mut events = File::open(cgroup.domain.join("cgroup.events"))?;
    loop {
        capture_shell_status(&mut shell, &mut shell_status)?;
        if shell_status.is_none() && process_is_outside(shell.id(), &cgroup.domain) {
            kill_shell(&mut shell, shell_pidfd.as_ref());
            capture_shell_status(&mut shell, &mut shell_status)?;
        }
        let populated = read_populated(&mut events).unwrap_or(true);
        if shell_status.is_some() && !populated {
            let code = shell_status.unwrap_or(1);
            status.line(&format!("result ok {code}"))?;
            return Ok(());
        }
        if Instant::now() >= deadline {
            return finish_cgroup_timeout(
                status,
                cgroup,
                &mut shell,
                &mut shell_status,
                shell_pidfd.as_ref(),
            );
        }
        let slice = deadline
            .saturating_duration_since(Instant::now())
            .min(Duration::from_millis(30));
        let _ = poll_fd(&events, slice);
    }
}

fn finish_cgroup_timeout(
    status: &mut StatusFile,
    cgroup: &ShellCgroup,
    shell: &mut Child,
    shell_status: &mut Option<i32>,
    shell_pidfd: Option<&OwnedFd>,
) -> io::Result<()> {
    let cleanup = kill_until_empty(&cgroup.domain, EMPTY_WAIT);
    if shell_status.is_none() {
        kill_shell(shell, shell_pidfd);
        let _ = wait_for_child(shell, EMPTY_WAIT)?;
        capture_shell_status(shell, shell_status)?;
    }
    let shell_alive = shell_status.is_none();
    if cleanup.empty && !shell_alive {
        if !cleanup.detail.is_empty() {
            status.line(&format!("cleanup {}", one_line_str(&cleanup.detail)))?;
        }
        status.line("result timeout")?;
    } else {
        let mut detail = if cleanup.detail.is_empty() && shell_alive {
            "shell still running".to_string()
        } else {
            cleanup.detail
        };
        if shell_alive && cgroup.ancestors_writable {
            if !detail.is_empty() {
                detail.push_str("; ");
            }
            detail.push_str("ancestor cgroup.procs is writable");
        }
        status.line(&format!("result unkillable {}", one_line_str(&detail)))?;
    }
    Ok(())
}

fn supervise_pid_namespace(
    status: &mut StatusFile,
    mut unshare: Child,
    status_path: &Path,
    deadline: Instant,
    cgroup_error: &str,
) -> io::Result<()> {
    let mut init_pidfd = None;
    let mut init_noted = false;
    loop {
        if init_pidfd.is_none() {
            if let Some(pid) = first_child_pid(unshare.id()) {
                if !init_noted {
                    status.line(&format!("init {pid}"))?;
                    init_noted = true;
                }
                init_pidfd = open_pidfd(pid);
            }
        }
        if let Some(exit) = unshare.try_wait()? {
            return finish_pid_namespace(
                status,
                status_path,
                exit_code(exit),
                false,
                cgroup_error,
                init_pidfd.as_ref(),
            );
        }
        if Instant::now() >= deadline {
            return kill_namespace(
                status,
                status_path,
                &mut unshare,
                cgroup_error,
                init_pidfd.as_ref(),
            );
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn kill_namespace(
    status: &mut StatusFile,
    status_path: &Path,
    unshare: &mut Child,
    cgroup_error: &str,
    init_pidfd: Option<&OwnedFd>,
) -> io::Result<()> {
    let mut opened = None;
    if init_pidfd.is_none() {
        if let Some(pid) = first_child_pid(unshare.id()) {
            status.line(&format!("init {pid}"))?;
            opened = open_pidfd(pid);
        }
    }
    let pidfd = init_pidfd.or(opened.as_ref());
    let Some(pidfd) = pidfd else {
        if unshare.try_wait()?.is_none() {
            let _ = unshare.kill();
        }
        let _ = wait_for_child(unshare, EMPTY_WAIT)?;
        status.line("result unkillable pid namespace init was not tracked")?;
        return Ok(());
    };
    let init_result = confirm_init_dead(pidfd, true);
    if unshare.try_wait()?.is_none() {
        let _ = unshare.kill();
    }
    let unshare_exit = wait_for_child(unshare, EMPTY_WAIT)?;
    match init_result {
        Ok(()) => {
            let code = unshare_exit.map_or(1, exit_code);
            finish_pid_namespace(status, status_path, code, true, cgroup_error, Some(pidfd))
        }
        Err(detail) => {
            status.line(&format!("result unkillable {}", one_line_str(&detail)))?;
            Ok(())
        }
    }
}

fn finish_pid_namespace(
    status: &mut StatusFile,
    status_path: &Path,
    code: i32,
    killed_for_timeout: bool,
    cgroup_error: &str,
    init_pidfd: Option<&OwnedFd>,
) -> io::Result<()> {
    // A normal exit still has to show that init is gone. Killing unshare is
    // not that proof: the namespace lives until its init process is gone.
    // `unshare` itself reaps the fork child on a normal exit, so a missing
    // pidfd is proof only when the exit code is not a signal.
    if let Some(pidfd) = init_pidfd {
        if let Err(detail) = confirm_init_dead(pidfd, false) {
            status.line(&format!("result unkillable {}", one_line_str(&detail)))?;
            return Ok(());
        }
    } else if killed_for_timeout || code > 128 {
        status.line("result unkillable pid namespace init was not tracked")?;
        return Ok(());
    }
    if !status_contains(status_path, "inner ready") {
        status.line(&format!(
            "result unavailable pid namespace did not start the supervisor (exit {code}); cgroup: {cgroup_error}"
        ))?;
        return Ok(());
    }
    if killed_for_timeout {
        status.line("result timeout")?;
    } else {
        status.line(&format!("result ok {code}"))?;
    }
    Ok(())
}

/// `force` sends `SIGKILL` when init is still alive. Otherwise this only waits.
fn confirm_init_dead(pidfd: &OwnedFd, force: bool) -> Result<(), String> {
    if pidfd_exited(pidfd, Duration::ZERO) {
        return Ok(());
    }
    if force {
        if let Err(err) = rustix::process::pidfd_send_signal(pidfd, rustix::process::Signal::KILL) {
            if !pidfd_exited(pidfd, Duration::ZERO) {
                return Err(format!("pid namespace init: {err}"));
            }
            return Ok(());
        }
    }
    if pidfd_exited(pidfd, EMPTY_WAIT) {
        Ok(())
    } else {
        Err("pid namespace init still running".to_string())
    }
}

fn wait_for_child(child: &mut Child, bound: Duration) -> io::Result<Option<ExitStatus>> {
    let deadline = Instant::now() + bound;
    loop {
        if let Some(exit) = child.try_wait()? {
            return Ok(Some(exit));
        }
        if Instant::now() >= deadline {
            return child.try_wait();
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn kill_shell(shell: &mut Child, pidfd: Option<&OwnedFd>) {
    if let Some(pidfd) = pidfd {
        let _ = rustix::process::pidfd_send_signal(pidfd, rustix::process::Signal::KILL);
    }
    if let Some(pid) = pid_from_raw(shell.id()) {
        // The shell called `setsid`, so its process-group id is its pid.
        // A group that does not exist yet returns `ESRCH`.
        let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
    }
    let _ = shell.kill();
}

fn capture_shell_status(shell: &mut Child, shell_status: &mut Option<i32>) -> io::Result<()> {
    if shell_status.is_none() {
        if let Some(exit) = shell.try_wait()? {
            *shell_status = Some(exit_code(exit));
        }
    }
    Ok(())
}

/// Pid 1 of a command's pid namespace. Reaps until `ECHILD`. Killing this
/// process kills every other process in the namespace.
fn run_pid1(status: &Path, command: &str) -> io::Result<()> {
    // Parent is `unshare`. If the supervisor kills `unshare`, or `unshare`
    // dies with the supervisor, this process gets `SIGKILL`.
    let _ = rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL));
    let _ = rustix::process::setsid();
    mark_pid1_ready(status)?;
    let mut shell = Command::new("/bin/sh");
    shell.arg("-c").arg(command).stdin(Stdio::null());
    crate::contain::setsid_child(&mut shell);
    let Ok(child) = shell.spawn() else {
        std::process::exit(126);
    };
    let shell_pid = rustix::process::Pid::from_child(&child);
    // `Child` does not wait or signal on drop. `wait` below is the reap.
    drop(child);
    let mut shell_status = None;
    loop {
        match rustix::process::wait(rustix::process::WaitOptions::empty()) {
            Ok(Some((pid, wait_status))) => {
                if pid == shell_pid {
                    shell_status = Some(wait_status_code(wait_status));
                }
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(rustix::io::Errno::CHILD) => break,
            Err(rustix::io::Errno::INTR) => {}
            Err(err) => return Err(err.into()),
        }
    }
    std::process::exit(shell_status.unwrap_or(126));
}

fn mark_pid1_ready(status: &Path) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).open(status)?;
    writeln!(file, "inner ready")?;
    file.sync_all()?;
    Ok(())
}

fn wait_status_code(status: rustix::process::WaitStatus) -> i32 {
    if let Some(code) = status.exit_status() {
        return code;
    }
    status
        .terminating_signal()
        .map_or(1, |signal| 128_i32.saturating_add(signal))
}

fn exit_code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    status
        .signal()
        .map_or(1, |signal| 128_i32.saturating_add(signal))
}

fn duration_from_millis(ms: u64) -> Duration {
    Duration::from_millis(ms.min(u64::MAX / 1_000_000))
}

fn one_line(err: &io::Error) -> String {
    one_line_str(&err.to_string())
}

fn one_line_str(text: &str) -> String {
    text.replace(['\n', '\r'], " ")
}

fn status_contains(path: &Path, line: &str) -> bool {
    fs::read_to_string(path).is_ok_and(|text| complete_lines(&text).any(|item| item == line))
}

struct StatusFile {
    file: File,
}

impl StatusFile {
    fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().append(true).open(path)?;
        Ok(Self { file })
    }

    fn line(&mut self, text: &str) -> io::Result<()> {
        writeln!(self.file, "{text}")?;
        self.file.sync_all()?;
        Ok(())
    }
}

struct ShellCgroup {
    domain: PathBuf,
    cmd_procs: CString,
    /// The command's uid can write an ancestor `cgroup.procs`, so it can leave
    /// this domain. Escape is detected from `/proc/<pid>/cgroup`.
    ancestors_writable: bool,
}

impl ShellCgroup {
    fn create() -> io::Result<Self> {
        let listing = fs::read_to_string("/proc/self/cgroup")
            .map_err(|err| io::Error::other(format!("cgroup listing: {err}")))?;
        let relative = listing
            .lines()
            .find_map(|line| line.strip_prefix("0::"))
            .filter(|path| !path.is_empty())
            .ok_or_else(|| io::Error::other("cgroup v2 hierarchy is not delegated"))?;
        let parent = PathBuf::from("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
        cleanup_leftovers(&parent);
        let domain = parent.join(format!("{CGROUP_PREFIX}{}", std::process::id()));
        fs::create_dir(&domain).map_err(|err| io::Error::other(format!("cgroup mkdir: {err}")))?;
        let created = Self {
            cmd_procs: cstring_path(&domain.join("cmd").join("cgroup.procs"))?,
            ancestors_writable: ancestor_procs_writable(&domain),
            domain,
        };
        if let Err(err) = created.make_children() {
            let _ = kill_until_empty(&created.domain, Duration::from_millis(0));
            return Err(err);
        }
        Ok(created)
    }

    fn make_children(&self) -> io::Result<()> {
        let supervisor = self.domain.join("supervisor");
        let cmd = self.domain.join("cmd");
        fs::create_dir(&supervisor)
            .map_err(|err| io::Error::other(format!("cgroup mkdir supervisor: {err}")))?;
        if let Err(err) = fs::create_dir(&cmd) {
            let _ = fs::remove_dir(&supervisor);
            return Err(io::Error::other(format!("cgroup mkdir cmd: {err}")));
        }
        if !self.domain.join("cgroup.kill").is_file()
            || !self.domain.join("cgroup.events").is_file()
        {
            return Err(io::Error::other(
                "cgroup.kill or cgroup.events is not available",
            ));
        }
        Ok(())
    }
}

impl Drop for ShellCgroup {
    fn drop(&mut self) {
        // Kill before remove. A leftover populated directory cannot be removed,
        // and a supervisor that is unwinding must not leave the command running.
        let _ = kill_until_empty(&self.domain, EMPTY_WAIT);
    }
}

fn cstring_path(path: &Path) -> io::Result<CString> {
    use std::os::unix::ffi::OsStrExt;
    CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::other("cgroup path contains a NUL"))
}

fn cleanup_leftovers(parent: &Path) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let Some(pid) = name.strip_prefix(CGROUP_PREFIX) else {
            continue;
        };
        let Ok(pid) = pid.parse::<u32>() else {
            continue;
        };
        // A live supervisor owns this directory. A reused pid is left alone
        // rather than killing whatever now has that id.
        if Path::new(&format!("/proc/{pid}")).exists() {
            continue;
        }
        let _ = kill_until_empty(&entry.path(), EMPTY_WAIT);
    }
}

fn ancestor_procs_writable(domain: &Path) -> bool {
    let uid = rustix::process::getuid().as_raw();
    let gid = rustix::process::getgid().as_raw();
    let mut current = domain.parent();
    while let Some(dir) = current {
        if procs_writable(&dir.join("cgroup.procs"), uid, gid) {
            return true;
        }
        if dir == Path::new("/sys/fs/cgroup") {
            break;
        }
        current = dir.parent();
    }
    false
}

fn procs_writable(path: &Path, uid: u32, gid: u32) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return true;
    };
    let mode = meta.mode();
    let owner_write = meta.uid() == uid && mode & 0o200 != 0;
    let group_write = meta.gid() == gid && mode & 0o020 != 0;
    let other_write = mode & 0o002 != 0;
    owner_write || group_write || other_write
}

pub(crate) fn cgroup_dir_is_ours(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if !name.starts_with(CGROUP_PREFIX) || !path.starts_with("/sys/fs/cgroup") {
        return false;
    }
    path.components()
        .all(|component| !matches!(component, std::path::Component::ParentDir))
}

/// Repeat `cgroup.kill` until `cgroup.events` says `populated 0`, then remove
/// the domain. Write and `rmdir` failures are returned in `detail`.
pub(crate) fn shutdown_cgroup(dir: &Path) -> CgroupCleanup {
    kill_until_empty(dir, EMPTY_WAIT)
}

fn kill_until_empty(dir: &Path, bound: Duration) -> CgroupCleanup {
    if !cgroup_dir_is_ours(dir) {
        return CgroupCleanup {
            empty: false,
            detail: "refusing to kill a cgroup outside the supervisor prefix".to_string(),
        };
    }
    let events_path = dir.join("cgroup.events");
    let mut events = match File::open(&events_path) {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return CgroupCleanup {
                empty: true,
                detail: remove_domain(dir),
            };
        }
        Err(err) => {
            return CgroupCleanup {
                empty: false,
                detail: format!("cgroup.events: {err}"),
            };
        }
    };
    let deadline = Instant::now() + bound;
    let mut kill_error = String::new();
    loop {
        let populated = match read_populated(&mut events) {
            Ok(value) => value,
            Err(err) => {
                return CgroupCleanup {
                    empty: false,
                    detail: format!("cgroup.events: {err}"),
                };
            }
        };
        if !populated {
            drop(events);
            return CgroupCleanup {
                empty: true,
                detail: join_detail(&kill_error, &remove_domain(dir)),
            };
        }
        if let Err(err) = fs::write(dir.join("cgroup.kill"), b"1") {
            let message = format!("cgroup.kill: {err}");
            if kill_error.is_empty() {
                kill_error = message;
            }
        }
        if Instant::now() >= deadline {
            let still = read_populated(&mut events).unwrap_or(true);
            if !still {
                drop(events);
                return CgroupCleanup {
                    empty: true,
                    detail: join_detail(&kill_error, &remove_domain(dir)),
                };
            }
            let detail = if kill_error.is_empty() {
                "cgroup still populated".to_string()
            } else {
                kill_error
            };
            return CgroupCleanup {
                empty: false,
                detail,
            };
        }
        let slice = deadline
            .saturating_duration_since(Instant::now())
            .min(Duration::from_millis(30));
        let _ = poll_fd(&events, slice);
    }
}

fn remove_domain(dir: &Path) -> String {
    let mut errors = Vec::new();
    for name in ["cmd", "supervisor"] {
        if let Err(err) = fs::remove_dir(dir.join(name)) {
            if err.kind() != io::ErrorKind::NotFound {
                errors.push(format!("rmdir {name}: {err}"));
            }
        }
    }
    if let Err(err) = fs::remove_dir(dir) {
        if err.kind() != io::ErrorKind::NotFound {
            errors.push(format!("rmdir {}: {err}", dir.display()));
        }
    }
    errors.join("; ")
}

fn join_detail(kill_error: &str, remove_error: &str) -> String {
    match (kill_error.is_empty(), remove_error.is_empty()) {
        (true, true) => String::new(),
        (false, true) => kill_error.to_string(),
        (true, false) => remove_error.to_string(),
        (false, false) => format!("{kill_error}; {remove_error}"),
    }
}

fn read_populated(file: &mut File) -> io::Result<bool> {
    file.seek(SeekFrom::Start(0))?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    populated_from_events(&text)
}

fn populated_from_events(text: &str) -> io::Result<bool> {
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("populated ") {
            return Ok(value.trim() != "0");
        }
    }
    Err(io::Error::other("cgroup.events has no populated field"))
}

fn poll_fd(file: &File, bound: Duration) -> io::Result<()> {
    let deadline = Instant::now() + bound;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        let timeout = Timespec::try_from(remaining).ok();
        let mut fds = [PollFd::new(
            file,
            PollFlags::PRI.union(PollFlags::IN).union(PollFlags::ERR),
        )];
        match rustix::event::poll(&mut fds, timeout.as_ref()) {
            Ok(_) => return Ok(()),
            Err(rustix::io::Errno::INTR) => {}
            Err(err) => return Err(err.into()),
        }
    }
}

fn process_is_outside(pid: u32, domain: &Path) -> bool {
    match fs::read_to_string(format!("/proc/{pid}/cgroup")) {
        Ok(text) => {
            let Some(relative) = text.lines().find_map(|line| line.strip_prefix("0::")) else {
                return true;
            };
            !under_domain(relative.trim(), domain)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => false,
        Err(_) => true,
    }
}

fn under_domain(relative: &str, domain: &Path) -> bool {
    let Ok(suffix) = domain.strip_prefix("/sys/fs/cgroup") else {
        return false;
    };
    let suffix = suffix.to_str().unwrap_or("");
    let domain_rel = if suffix.is_empty() {
        "/".to_string()
    } else {
        format!("/{suffix}")
    };
    relative == domain_rel || relative.starts_with(&format!("{domain_rel}/"))
}

fn first_child_pid(pid: u32) -> Option<u32> {
    let text = fs::read_to_string(format!("/proc/{pid}/task/{pid}/children")).ok()?;
    text.split_whitespace().next()?.parse().ok()
}

pub(crate) fn open_pidfd(pid: u32) -> Option<OwnedFd> {
    let pid = pid_from_raw(pid)?;
    rustix::process::pidfd_open(pid, rustix::process::PidfdFlags::empty()).ok()
}

pub(crate) fn pidfd_exited(pidfd: &OwnedFd, bound: Duration) -> bool {
    let deadline = Instant::now() + bound;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let timeout = Timespec::try_from(remaining).ok();
        let mut fds = [PollFd::new(pidfd, PollFlags::IN)];
        match rustix::event::poll(&mut fds, timeout.as_ref()) {
            Ok(0) => return false,
            Ok(_) => return true,
            Err(rustix::io::Errno::INTR) => {}
            Err(_) => return false,
        }
        if Instant::now() >= deadline {
            return false;
        }
    }
}

/// Kill `pidfd` if the process is still alive and wait until it exits.
pub(crate) fn ensure_pidfd_dead(pidfd: &OwnedFd, bound: Duration) -> Result<(), String> {
    if pidfd_exited(pidfd, Duration::ZERO) {
        return Ok(());
    }
    if let Err(err) = rustix::process::pidfd_send_signal(pidfd, rustix::process::Signal::KILL) {
        if pidfd_exited(pidfd, Duration::ZERO) {
            return Ok(());
        }
        return Err(format!("pidfd kill: {err}"));
    }
    if pidfd_exited(pidfd, bound) {
        Ok(())
    } else {
        Err("process still running after SIGKILL".to_string())
    }
}

fn pid_from_raw(pid: u32) -> Option<rustix::process::Pid> {
    i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
}

pub(crate) fn parse_supervisor_result(text: &str) -> Option<SupervisorOutcome> {
    let mut outcome = None;
    for line in complete_lines(text) {
        let mut parts = line.splitn(3, ' ');
        outcome = match (parts.next(), parts.next(), parts.next()) {
            (Some("result"), Some("ok"), Some(rest)) => rest
                .split_whitespace()
                .next()
                .and_then(|code| code.parse().ok())
                .map(SupervisorOutcome::Finished),
            (Some("result"), Some("timeout"), _) => Some(SupervisorOutcome::Timeout),
            (Some("result"), Some("unkillable"), detail) => Some(SupervisorOutcome::Unkillable {
                detail: detail.unwrap_or("").to_string(),
            }),
            (Some("result"), Some("unavailable"), detail) => Some(SupervisorOutcome::Unavailable {
                detail: detail.unwrap_or("").to_string(),
            }),
            (Some("result"), Some("error"), Some(message)) => {
                Some(SupervisorOutcome::Failed(message.to_string()))
            }
            _ => outcome,
        };
    }
    outcome
}

pub(crate) fn parse_cleanup_notes(text: &str) -> String {
    complete_lines(text)
        .filter_map(|line| line.strip_prefix("cleanup "))
        .collect::<Vec<_>>()
        .join("; ")
}

pub(crate) fn parse_live_containment(text: &str) -> Option<LiveContainment> {
    let mut live = LiveContainment::default();
    let mut seen = false;
    for line in complete_lines(text) {
        let mut parts = line.splitn(3, ' ');
        match (parts.next(), parts.next(), parts.next()) {
            (Some("mode"), Some("cgroup"), Some(path)) if path.starts_with('/') => {
                live.cgroup = Some(PathBuf::from(path));
                seen = true;
            }
            (Some("mode"), Some("pidns"), Some(pid)) => {
                if let Ok(pid) = pid.parse() {
                    live.unshare = Some(pid);
                    seen = true;
                }
            }
            (Some("mode"), Some("none"), _) => {
                live = LiveContainment::default();
                seen = false;
            }
            (Some("shell"), Some(pid), None) => {
                if let Ok(pid) = pid.parse() {
                    live.shell = Some(pid);
                }
            }
            (Some("init"), Some(pid), None) => {
                if let Ok(pid) = pid.parse() {
                    live.init = Some(pid);
                }
            }
            _ => {}
        }
    }
    seen.then_some(live)
}

fn complete_lines(text: &str) -> impl Iterator<Item = &str> {
    let body = if text.ends_with('\n') {
        text
    } else if let Some(index) = text.rfind('\n') {
        &text[..=index]
    } else {
        ""
    };
    body.lines()
}

#[cfg(test)]
mod tests {
    use super::{
        parse_live_containment, parse_supervisor_result, populated_from_events, LiveContainment,
        SupervisorOutcome,
    };

    #[test]
    fn timeout_is_a_flag_and_exit_124_is_a_status() {
        assert!(matches!(
            parse_supervisor_result(
                "mode cgroup /sys/fs/cgroup/bash-supervisor-1\nresult timeout\n"
            ),
            Some(SupervisorOutcome::Timeout)
        ));
        assert!(matches!(
            parse_supervisor_result("result ok 124\n"),
            Some(SupervisorOutcome::Finished(124))
        ));
    }

    #[test]
    fn unkillable_is_not_an_exit_code() {
        assert!(matches!(
            parse_supervisor_result("result unkillable cgroup.kill: busy\n"),
            Some(SupervisorOutcome::Unkillable { .. })
        ));
        assert!(parse_supervisor_result("result ok 127\n")
            .is_some_and(|outcome| { matches!(outcome, SupervisorOutcome::Finished(127)) }));
    }

    #[test]
    fn partial_trailing_line_is_ignored() {
        assert!(parse_supervisor_result("result timeout").is_none());
        let live = parse_live_containment("mode pidns 42\ninit 43\nresult time")
            .expect("complete pidns line");
        assert_eq!(live.unshare, Some(42));
        assert_eq!(live.init, Some(43));
    }

    #[test]
    fn emptiness_is_populated_zero() {
        assert!(!populated_from_events("populated 0\nfrozen 0\n").expect("populated 0"));
        assert!(populated_from_events("populated 1\nfrozen 0\n").expect("populated 1"));
        assert!(populated_from_events("frozen 0\npopulated 1\n").expect("reordered"));
        assert!(populated_from_events("frozen 0\n").is_err());
    }

    #[test]
    fn fallback_keeps_the_cgroup_path_and_the_namespace() {
        let live = parse_live_containment(
            "mode cgroup /sys/fs/cgroup/bash-supervisor-7\nmode pidns 8\nshell 9\ninit 10\n",
        )
        .expect("both containers");
        assert_eq!(
            live.cgroup.as_deref(),
            Some(std::path::Path::new("/sys/fs/cgroup/bash-supervisor-7"))
        );
        assert_eq!(live.shell, Some(9));
        assert_eq!(live.unshare, Some(8));
        assert_eq!(live.init, Some(10));
        let _ = LiveContainment::default();
    }
}
