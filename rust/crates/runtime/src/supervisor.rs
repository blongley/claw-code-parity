//! Contain one timed shell command in a pid namespace, then tear it down.
//!
//! The supervisor is its own session. It creates a user and pid namespace with
//! `unshare` in this process, forks, and opens the child's pidfd before the
//! child is allowed to exec the command. The child is pid 1 of that namespace.
//! Timeout and teardown send `SIGKILL` through the pidfd and wait on it. The
//! kernel then kills every process in the namespace. `setsid` and a move into
//! another cgroup cannot leave it.
//!
//! If the pidfd was never opened, the result is `unkillable`. A normal exit is
//! not proof that the namespace is gone. If the namespace cannot be created,
//! the command is not started.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec};
use rustix::process::WaitOptions;

/// How long teardown waits for the namespace init to exit.
pub(crate) const EMPTY_WAIT: Duration = Duration::from_secs(2);

/// Status text for a process that survived teardown. Not an exit code.
pub(crate) const UNKILLABLE_STATUS: &str = "processes could not be killed";

/// Detail used when teardown has no pidfd to signal.
pub(crate) const INIT_NOT_TRACKED: &str = "pid namespace init was not tracked";

const RELEASED: u8 = 1;

#[derive(Debug)]
pub(crate) enum SupervisorOutcome {
    Finished(i32),
    Timeout,
    Unkillable { detail: String },
    Unavailable { detail: String },
    Failed(String),
}

pub(crate) enum LaunchError {
    Unavailable(io::Error),
    Run(io::Error),
}

/// Run `command` only after `setup` succeeds. A namespace error never starts it.
pub(crate) fn launch_contained<T>(
    setup: Result<T, io::Error>,
    command: impl FnOnce(T) -> io::Result<()>,
) -> Result<(), LaunchError> {
    match setup {
        Ok(ready) => command(ready).map_err(LaunchError::Run),
        Err(err) => Err(LaunchError::Unavailable(err)),
    }
}

#[derive(Debug)]
pub(crate) enum ExitDecision {
    Report(SupervisorOutcome),
    MissingStatus,
}

/// A missing init pidfd is `unkillable`, including when a wrapper exit is 0.
/// Setup failures stay `unavailable` so the parent can refuse without claiming
/// a process survived.
pub(crate) fn decide_exit(outcome: Option<SupervisorOutcome>, init_recorded: bool) -> ExitDecision {
    if !init_recorded {
        return ExitDecision::Report(match outcome {
            Some(SupervisorOutcome::Unavailable { detail }) => {
                SupervisorOutcome::Unavailable { detail }
            }
            Some(SupervisorOutcome::Failed(message)) => SupervisorOutcome::Failed(message),
            Some(SupervisorOutcome::Unkillable { detail }) => {
                SupervisorOutcome::Unkillable { detail }
            }
            Some(SupervisorOutcome::Finished(_) | SupervisorOutcome::Timeout) | None => {
                SupervisorOutcome::Unkillable {
                    detail: INIT_NOT_TRACKED.to_string(),
                }
            }
        });
    }
    match outcome {
        Some(outcome) => ExitDecision::Report(outcome),
        None => ExitDecision::MissingStatus,
    }
}

/// Apply the pidfd wait to a parsed status. `init_dead` is false when the
/// kill or the wait failed, or when no pidfd was held.
pub(crate) fn finalize_exit(
    outcome: Option<SupervisorOutcome>,
    init_recorded: bool,
    init_dead: bool,
    release_detail: &str,
) -> Result<SupervisorOutcome, ()> {
    match decide_exit(outcome, init_recorded) {
        ExitDecision::MissingStatus => Err(()),
        ExitDecision::Report(SupervisorOutcome::Unavailable { detail }) => {
            Ok(SupervisorOutcome::Unavailable { detail })
        }
        ExitDecision::Report(SupervisorOutcome::Failed(message)) => {
            Ok(SupervisorOutcome::Failed(message))
        }
        ExitDecision::Report(SupervisorOutcome::Unkillable { detail }) => {
            Ok(SupervisorOutcome::Unkillable {
                detail: join_detail(&detail, release_detail),
            })
        }
        ExitDecision::Report(_) if !init_dead => Ok(SupervisorOutcome::Unkillable {
            detail: if release_detail.is_empty() {
                "pid namespace init still running".to_string()
            } else {
                release_detail.to_string()
            },
        }),
        ExitDecision::Report(other) => Ok(other),
    }
}

pub(crate) fn bash_supervisor_main() -> i32 {
    match supervise() {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("bash supervisor: {err}");
            1
        }
    }
}

fn supervise() -> io::Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let (status_path, timeout_ms, command) = parse_outer(&args)?;
    let mut status = StatusFile::open(&status_path)?;
    match prepare_session() {
        SessionStart::Ready => {}
        SessionStart::ParentExited => {
            status.line("result unavailable parent exited before the supervisor started")?;
            return Ok(());
        }
        SessionStart::Failed(err) => {
            status.line(&format!("result error {}", one_line(&err)))?;
            return Ok(());
        }
    }

    let deadline = Instant::now() + duration_from_millis(timeout_ms);
    let started = launch_contained(crate::contain::enter_user_and_pid_namespace(), |()| {
        run_in_namespace(&mut status, &status_path, &command, deadline)
    });
    match started {
        Ok(()) => Ok(()),
        Err(LaunchError::Unavailable(err)) => {
            status.line(&format!("result unavailable {}", one_line(&err)))?;
            Ok(())
        }
        Err(LaunchError::Run(err)) => Err(err),
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

fn run_in_namespace(
    status: &mut StatusFile,
    status_path: &Path,
    command: &str,
    deadline: Instant,
) -> io::Result<()> {
    let (mut reader, writer) = io::pipe()?;
    match crate::contain::fork_process()? {
        crate::contain::ForkResult::Child => {
            drop(writer);
            if let Err(err) = arm_parent_death() {
                record_note(status_path, &format!("parent-death signal: {err}"));
            }
            let code = if wait_until_released(&mut reader) {
                drop(reader);
                command_as_init(status_path, command)
            } else {
                126
            };
            std::process::exit(code);
        }
        crate::contain::ForkResult::Parent(pid) => {
            drop(reader);
            let pidfd = match open_pidfd(pid) {
                Ok(fd) => fd,
                Err(err) => {
                    let kill_error = kill_and_reap(pid);
                    drop(writer);
                    let detail = match kill_error {
                        Ok(()) => format!("pidfd: {err}"),
                        Err(kill_error) => format!("pidfd: {err}; {kill_error}"),
                    };
                    status.line(&format!("result unkillable {}", one_line_str(&detail)))?;
                    return Ok(());
                }
            };
            let mut init = TrackedInit {
                pid,
                pidfd,
                gate: writer,
                status_path: status_path.to_path_buf(),
                torn_down: false,
                reaped: false,
            };
            // The pidfd is open. Record it before the child may exec.
            status.line(&format!("init {}", init.pid))?;
            if let Err(err) = init.release() {
                let mut detail = format!("release: {err}");
                if let Err(kill_error) = init.teardown() {
                    detail.push_str("; ");
                    detail.push_str(&kill_error);
                }
                status.line(&format!("result unkillable {}", one_line_str(&detail)))?;
                return Ok(());
            }
            supervise_init(status, &mut init, status_path, deadline)
        }
    }
}

fn arm_parent_death() -> io::Result<()> {
    rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL))
        .map_err(io::Error::from)
}

fn wait_until_released(reader: &mut impl Read) -> bool {
    let mut buf = [0_u8; 1];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => return false,
            Ok(_) => return buf[0] == RELEASED,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return false,
        }
    }
}

struct TrackedInit {
    pid: u32,
    pidfd: OwnedFd,
    gate: std::io::PipeWriter,
    status_path: PathBuf,
    torn_down: bool,
    reaped: bool,
}

impl TrackedInit {
    fn release(&mut self) -> io::Result<()> {
        self.gate.write_all(&[RELEASED])?;
        self.gate.flush()?;
        Ok(())
    }

    fn teardown(&mut self) -> Result<(), String> {
        if self.torn_down {
            return Ok(());
        }
        let killed = ensure_pidfd_dead(&self.pidfd, EMPTY_WAIT);
        let reaped = if self.reaped {
            Ok(())
        } else {
            reap_bounded(self.pid).map(|_| ())
        };
        self.reaped = true;
        self.torn_down = true;
        match (killed, reaped) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(kill_error), Ok(())) => Err(kill_error),
            (Ok(()), Err(wait_error)) => Err(wait_error),
            (Err(kill_error), Err(wait_error)) => Err(format!("{kill_error}; {wait_error}")),
        }
    }
}

impl Drop for TrackedInit {
    fn drop(&mut self) {
        if let Err(err) = self.teardown() {
            eprintln!("bash supervisor teardown: {err}");
            record_note(&self.status_path, &err);
        }
    }
}

fn supervise_init(
    status: &mut StatusFile,
    init: &mut TrackedInit,
    status_path: &Path,
    deadline: Instant,
) -> io::Result<()> {
    loop {
        match try_reap(init.pid) {
            Ok(Some(code)) => {
                init.reaped = true;
                return finish_reaped(status, init, status_path, code, false);
            }
            Ok(None) => {}
            Err(err) => {
                if let Err(kill_error) = init.teardown() {
                    status.line(&format!(
                        "result unkillable {}",
                        one_line_str(&format!("{err}; {kill_error}"))
                    ))?;
                } else {
                    status.line(&format!("result unkillable {}", one_line_str(&err)))?;
                }
                return Ok(());
            }
        }
        if Instant::now() >= deadline {
            return finish_timeout(status, init, status_path);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn finish_reaped(
    status: &mut StatusFile,
    init: &mut TrackedInit,
    status_path: &Path,
    code: i32,
    timed_out: bool,
) -> io::Result<()> {
    init.torn_down = true;
    match pidfd_has_exited(&init.pidfd, EMPTY_WAIT) {
        Err(err) => {
            status.line(&format!("result unkillable {}", one_line_str(&err)))?;
            Ok(())
        }
        Ok(false) => {
            status.line("result unkillable pid namespace init still running")?;
            Ok(())
        }
        Ok(true) => write_finished(status, status_path, code, timed_out),
    }
}

fn finish_timeout(
    status: &mut StatusFile,
    init: &mut TrackedInit,
    status_path: &Path,
) -> io::Result<()> {
    if let Err(err) = init.teardown() {
        status.line(&format!("result unkillable {}", one_line_str(&err)))?;
        return Ok(());
    }
    write_finished(status, status_path, 1, true)
}

fn write_finished(
    status: &mut StatusFile,
    status_path: &Path,
    code: i32,
    timed_out: bool,
) -> io::Result<()> {
    if let Some(detail) = note_with_prefix(status_path, "init-wait ") {
        status.line(&format!("result unkillable {}", one_line_str(&detail)))?;
        return Ok(());
    }
    if !status_contains(status_path, "inner ready") {
        status.line(&format!(
            "result unavailable pid namespace did not start the command (exit {code})"
        ))?;
        return Ok(());
    }
    if timed_out {
        status.line("result timeout")?;
    } else {
        status.line(&format!("result ok {code}"))?;
    }
    Ok(())
}

/// Pid 1 of the command's pid namespace. Reaps until `ECHILD`. Killing this
/// process kills every other process in the namespace.
fn command_as_init(status: &Path, command: &str) -> i32 {
    if let Err(err) = rustix::process::setsid() {
        record_note(status, &format!("setsid: {err}"));
    }
    if mark_ready(status).is_err() {
        return 126;
    }
    let mut shell = Command::new("/bin/sh");
    shell.arg("-c").arg(command).stdin(Stdio::null());
    crate::contain::setsid_child(&mut shell);
    let Ok(child) = shell.spawn() else {
        return 126;
    };
    let shell_pid = rustix::process::Pid::from_child(&child);
    // `Child` does not wait or signal on drop. The loop below is the reap.
    drop(child);
    let mut shell_status = None;
    loop {
        match rustix::process::wait(WaitOptions::empty()) {
            Ok(Some((pid, wait_status))) => {
                if pid == shell_pid {
                    shell_status = Some(wait_status_code(wait_status));
                }
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(rustix::io::Errno::CHILD) => break,
            Err(rustix::io::Errno::INTR) => {}
            Err(err) => {
                record_init_wait(status, &err.to_string());
                return 126;
            }
        }
    }
    shell_status.unwrap_or(126)
}

fn mark_ready(status: &Path) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).open(status)?;
    writeln!(file, "inner ready")?;
    file.sync_all()?;
    Ok(())
}

fn record_init_wait(status: &Path, detail: &str) {
    let line = format!("init-wait {}", one_line_str(detail));
    match OpenOptions::new().append(true).open(status) {
        Ok(mut file) => {
            if let Err(err) = writeln!(file, "{line}") {
                eprintln!("{line}; status write: {err}");
            }
        }
        Err(err) => eprintln!("{line}; status open: {err}"),
    }
}

fn record_note(status: &Path, detail: &str) {
    let line = format!("cleanup {}", one_line_str(detail));
    match OpenOptions::new().append(true).open(status) {
        Ok(mut file) => {
            if let Err(err) = writeln!(file, "{line}") {
                eprintln!("{line}; status write: {err}");
            }
        }
        Err(err) => eprintln!("{line}; status open: {err}"),
    }
}

fn wait_status_code(status: rustix::process::WaitStatus) -> i32 {
    if let Some(code) = status.exit_status() {
        return code;
    }
    status
        .terminating_signal()
        .map_or(1, |signal| 128_i32.saturating_add(signal))
}

fn try_reap(pid: u32) -> Result<Option<i32>, String> {
    let Some(pid) = pid_from_raw(pid) else {
        return Err("wait: pid is not a process id".to_string());
    };
    match rustix::process::waitpid(Some(pid), WaitOptions::NOHANG) {
        Ok(Some((_, wait_status))) => Ok(Some(wait_status_code(wait_status))),
        Ok(None) | Err(rustix::io::Errno::INTR) => Ok(None),
        Err(rustix::io::Errno::CHILD) => {
            Err("wait: pid namespace init was not a child".to_string())
        }
        Err(err) => Err(format!("wait: {err}")),
    }
}

fn reap_bounded(pid: u32) -> Result<Option<i32>, String> {
    let deadline = Instant::now() + EMPTY_WAIT;
    loop {
        match try_reap(pid) {
            Ok(Some(code)) => return Ok(Some(code)),
            Ok(None) if Instant::now() >= deadline => {
                return Err("wait: pid namespace init still running".to_string());
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(err) => return Err(err),
        }
    }
}

fn kill_and_reap(pid: u32) -> Result<(), String> {
    let Some(raw) = pid_from_raw(pid) else {
        return Err("kill: pid is not a process id".to_string());
    };
    let killed = match rustix::process::kill_process(raw, rustix::process::Signal::KILL) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(err) => Err(format!("kill: {err}")),
    };
    let waited = reap_bounded(pid).map(|_| ());
    match (killed, waited) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(kill_error), Ok(())) => Err(kill_error),
        (Ok(()), Err(wait_error)) => Err(wait_error),
        (Err(kill_error), Err(wait_error)) => Err(format!("{kill_error}; {wait_error}")),
    }
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

fn note_with_prefix(path: &Path, prefix: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let notes = complete_lines(&text)
        .filter_map(|line| line.strip_prefix(prefix))
        .collect::<Vec<_>>();
    if notes.is_empty() {
        None
    } else {
        Some(notes.join("; "))
    }
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

pub(crate) fn open_pidfd(pid: u32) -> io::Result<OwnedFd> {
    let pid = pid_from_raw(pid).ok_or_else(|| io::Error::other("pid is not a process id"))?;
    rustix::process::pidfd_open(pid, rustix::process::PidfdFlags::empty()).map_err(io::Error::from)
}

fn pidfd_has_exited(pidfd: &OwnedFd, bound: Duration) -> Result<bool, String> {
    let deadline = Instant::now() + bound;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let timeout = Timespec::try_from(remaining).ok();
        let mut fds = [PollFd::new(pidfd, PollFlags::IN)];
        match rustix::event::poll(&mut fds, timeout.as_ref()) {
            Ok(0) => return Ok(false),
            Ok(_) => return Ok(true),
            Err(rustix::io::Errno::INTR) => {
                if Instant::now() >= deadline {
                    return Ok(false);
                }
            }
            Err(err) => return Err(format!("pidfd wait: {err}")),
        }
    }
}

/// Kill `pidfd` if the process is still alive and wait until it exits.
pub(crate) fn ensure_pidfd_dead(pidfd: &OwnedFd, bound: Duration) -> Result<(), String> {
    match pidfd_has_exited(pidfd, Duration::ZERO) {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(err) => return Err(err),
    }
    if let Err(err) = rustix::process::pidfd_send_signal(pidfd, rustix::process::Signal::KILL) {
        if err != rustix::io::Errno::SRCH {
            return Err(format!("pidfd kill: {err}"));
        }
        return match pidfd_has_exited(pidfd, Duration::ZERO) {
            Ok(true) => Ok(()),
            Ok(false) => Err(format!("pidfd kill: {err}")),
            Err(wait_error) => Err(format!("pidfd kill: {err}; {wait_error}")),
        };
    }
    match pidfd_has_exited(pidfd, bound) {
        Ok(true) => Ok(()),
        Ok(false) => Err("process still running after SIGKILL".to_string()),
        Err(err) => Err(err),
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

/// The first complete `init <pid>` line. Later lines cannot replace it.
pub(crate) fn parse_init_pid(text: &str) -> Option<u32> {
    complete_lines(text).find_map(|line| {
        let rest = line.strip_prefix("init ")?;
        if rest.contains(' ') {
            return None;
        }
        rest.parse().ok()
    })
}

fn join_detail(left: &str, right: &str) -> String {
    match (left.is_empty(), right.is_empty()) {
        (true, true) => String::new(),
        (false, true) => left.to_string(),
        (true, false) => right.to_string(),
        (false, false) => format!("{left}; {right}"),
    }
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
    use std::io;

    use super::{
        decide_exit, finalize_exit, launch_contained, parse_init_pid, parse_supervisor_result,
        ExitDecision, LaunchError, SupervisorOutcome, INIT_NOT_TRACKED,
    };

    #[test]
    fn timeout_is_a_flag_and_exit_124_is_a_status() {
        assert!(matches!(
            parse_supervisor_result("init 4\nresult timeout\n"),
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
            parse_supervisor_result("result unkillable pidfd kill: busy\n"),
            Some(SupervisorOutcome::Unkillable { .. })
        ));
        assert!(parse_supervisor_result("result ok 127\n")
            .is_some_and(|outcome| { matches!(outcome, SupervisorOutcome::Finished(127)) }));
    }

    #[test]
    fn partial_trailing_line_is_ignored() {
        assert!(parse_supervisor_result("result timeout").is_none());
        assert_eq!(parse_init_pid("init 43\nresult time"), Some(43));
        assert_eq!(parse_init_pid("init 43"), None);
        assert_eq!(parse_init_pid("init 43\ninit 1\n"), Some(43));
    }

    #[test]
    fn unrecorded_init_is_unkillable_even_if_the_wrapper_exits_zero() {
        match decide_exit(Some(SupervisorOutcome::Finished(0)), false) {
            ExitDecision::Report(SupervisorOutcome::Unkillable { detail }) => {
                assert_eq!(detail, INIT_NOT_TRACKED);
            }
            other => panic!("expected unkillable, got {other:?}"),
        }
        assert!(matches!(
            decide_exit(Some(SupervisorOutcome::Timeout), false),
            ExitDecision::Report(SupervisorOutcome::Unkillable { .. })
        ));
        assert!(matches!(
            decide_exit(Some(SupervisorOutcome::Finished(0)), true),
            ExitDecision::Report(SupervisorOutcome::Finished(0))
        ));
        let refused = finalize_exit(
            Some(SupervisorOutcome::Unavailable {
                detail: "unshare: EPERM".to_string(),
            }),
            false,
            false,
            "",
        );
        assert!(matches!(refused, Ok(SupervisorOutcome::Unavailable { .. })));
        let killed = finalize_exit(
            Some(SupervisorOutcome::Finished(0)),
            true,
            false,
            "pidfd kill: busy",
        );
        match killed {
            Ok(SupervisorOutcome::Unkillable { detail }) => {
                assert_eq!(detail, "pidfd kill: busy");
            }
            other => panic!("expected the kill error, got {other:?}"),
        }
    }

    #[test]
    fn namespace_setup_failure_does_not_run_the_command() {
        let mut ran = false;
        let error = launch_contained(Err(io::Error::other("unshare: EPERM")), |()| {
            ran = true;
            Ok(())
        })
        .expect_err("a pid namespace failure must fail closed");
        assert!(!ran, "the command ran without a pid namespace");
        assert!(matches!(error, LaunchError::Unavailable(_)));
    }
}
