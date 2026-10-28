//! Contain one timed shell command, then tear it down.
//!
//! The supervisor is its own session. It starts the command only after a
//! cgroup v2 exists for it, or inside a pid namespace. Teardown writes
//! `cgroup.kill` (or kills the namespace init by killing `unshare`) and waits
//! until that container is empty. A process that is still there after the
//! wait is `unkillable`, which the parent reports on its own. It is never an
//! exit status of 127.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use std::os::unix::process::ExitStatusExt;

/// How long teardown waits for the cgroup or pid namespace to become empty.
const EMPTY_WAIT: Duration = Duration::from_secs(2);

/// Status text for a process that survived teardown. Not an exit code.
pub(crate) const UNKILLABLE_STATUS: &str = "processes could not be killed";

pub(crate) enum SupervisorOutcome {
    Finished(i32),
    Timeout,
    Unkillable,
    Unavailable { detail: String },
    Failed(String),
}

pub(crate) enum LiveContainment {
    Cgroup(PathBuf),
    PidNamespace(u32),
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
            status.line(&format!("mode cgroup {}", cgroup.dir.display()))?;
            match spawn_shell(&command, &cgroup) {
                Ok(shell) => supervise_cgroup(&mut status, &cgroup, shell, deadline),
                Err(err) => {
                    // The command did not start. Do not fall back to a pid
                    // namespace after a cgroup was created.
                    status.line(&format!(
                        "result error command was not started: {}",
                        one_line(&err)
                    ))?;
                    Ok(())
                }
            }
        }
        Err(cgroup_err) => match spawn_pid_namespace(&status_path, &command) {
            Ok(unshare) => {
                status.line(&format!("mode pidns {}", unshare.id()))?;
                supervise_pid_namespace(
                    &mut status,
                    unshare,
                    &status_path,
                    deadline,
                    &one_line(&cgroup_err),
                )
            }
            Err(pidns_err) => {
                status.line("mode none")?;
                status.line(&format!(
                    "result unavailable cgroup: {}; pidns: {}",
                    one_line(&cgroup_err),
                    one_line(&pidns_err)
                ))?;
                Ok(())
            }
        },
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
    crate::contain::isolate_shell(&mut shell, &cgroup.procs_path());
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
    unshare.spawn()
}

fn supervise_cgroup(
    status: &mut StatusFile,
    cgroup: &ShellCgroup,
    mut shell: Child,
    deadline: Instant,
) -> io::Result<()> {
    let mut shell_status = None;
    loop {
        capture_shell_status(&mut shell, &mut shell_status)?;
        if shell_status.is_some() && cgroup.procs_empty() {
            let code = shell_status.unwrap_or(1);
            status.line(&format!("result ok {code}"))?;
            return Ok(());
        }
        if Instant::now() >= deadline {
            return finish_cgroup_timeout(status, cgroup, &mut shell, &mut shell_status);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn finish_cgroup_timeout(
    status: &mut StatusFile,
    cgroup: &ShellCgroup,
    shell: &mut Child,
    shell_status: &mut Option<i32>,
) -> io::Result<()> {
    cgroup.kill_all();
    if shell_status.is_none() {
        match shell.try_wait() {
            Ok(Some(exit)) => *shell_status = Some(exit_code(exit)),
            Ok(None) => {
                // The direct child is outside the cgroup only when moving it
                // failed, and that already aborted spawn. Kill it anyway so a
                // stopped shell cannot outlive the deadline.
                let _ = shell.kill();
            }
            Err(err) => return Err(err),
        }
    }
    let empty = wait_for_cgroup(cgroup, shell, shell_status, EMPTY_WAIT)?;
    if empty && shell_status.is_some() {
        status.line("result timeout")?;
    } else {
        status.line("result unkillable")?;
    }
    Ok(())
}

fn wait_for_cgroup(
    cgroup: &ShellCgroup,
    shell: &mut Child,
    shell_status: &mut Option<i32>,
    bound: Duration,
) -> io::Result<bool> {
    let deadline = Instant::now() + bound;
    loop {
        capture_shell_status(shell, shell_status)?;
        if shell_status.is_some() && cgroup.procs_empty() {
            return Ok(true);
        }
        if Instant::now() >= deadline {
            capture_shell_status(shell, shell_status)?;
            return Ok(shell_status.is_some() && cgroup.procs_empty());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn capture_shell_status(shell: &mut Child, shell_status: &mut Option<i32>) -> io::Result<()> {
    if shell_status.is_none() {
        if let Some(exit) = shell.try_wait()? {
            *shell_status = Some(exit_code(exit));
        }
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
    loop {
        if let Some(exit) = unshare.try_wait()? {
            return finish_pid_namespace(status, status_path, exit_code(exit), false, cgroup_error);
        }
        if Instant::now() >= deadline {
            if unshare.try_wait()?.is_none() {
                let _ = unshare.kill();
            }
            let exited = wait_for_child(&mut unshare, EMPTY_WAIT)?;
            return if let Some(exit) = exited {
                finish_pid_namespace(status, status_path, exit_code(exit), true, cgroup_error)
            } else {
                status.line("result unkillable")?;
                Ok(())
            };
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn finish_pid_namespace(
    status: &mut StatusFile,
    status_path: &Path,
    code: i32,
    killed_for_timeout: bool,
    cgroup_error: &str,
) -> io::Result<()> {
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

/// Pid 1 of a command's pid namespace. Reaps until `ECHILD`. Killing this
/// process (via `unshare --kill-child`) kills every other process in the
/// namespace.
fn run_pid1(status: &Path, command: &str) -> io::Result<()> {
    let _ = rustix::process::setsid();
    mark_pid1_ready(status)?;
    let mut shell = Command::new("/bin/sh");
    shell.arg("-c").arg(command).stdin(Stdio::null());
    crate::contain::setsid_child(&mut shell);
    let child = match shell.spawn() {
        Ok(child) => child,
        Err(err) => {
            // 126, not 127: a failed exec must not look like the old
            // unkillable mapping.
            let _ = err;
            std::process::exit(126);
        }
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
    err.to_string().replace(['\n', '\r'], " ")
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
    dir: PathBuf,
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
        let dir = parent.join(format!("bash-supervisor-{}", std::process::id()));
        fs::create_dir(&dir).map_err(|err| io::Error::other(format!("cgroup mkdir: {err}")))?;
        let created = Self { dir };
        if !created.dir.join("cgroup.kill").is_file() || !created.dir.join("cgroup.procs").is_file()
        {
            return Err(io::Error::other("cgroup.kill is not available"));
        }
        Ok(created)
    }

    fn procs_path(&self) -> PathBuf {
        self.dir.join("cgroup.procs")
    }

    fn procs_empty(&self) -> bool {
        procs_empty(&self.dir)
    }

    fn kill_all(&self) {
        kill_cgroup(&self.dir);
    }
}

impl Drop for ShellCgroup {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.dir);
    }
}

pub(crate) fn cgroup_dir_is_ours(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with("bash-supervisor-") && path.starts_with("/sys/fs/cgroup")
}

pub(crate) fn kill_cgroup(dir: &Path) {
    if cgroup_dir_is_ours(dir) {
        let _ = fs::write(dir.join("cgroup.kill"), b"1");
    }
}

pub(crate) fn procs_empty(dir: &Path) -> bool {
    let Ok(text) = fs::read_to_string(dir.join("cgroup.procs")) else {
        return false;
    };
    text.split_whitespace().next().is_none()
}

pub(crate) fn wait_cgroup_empty(dir: &Path, bound: Duration) -> bool {
    let deadline = Instant::now() + bound;
    loop {
        if procs_empty(dir) {
            return true;
        }
        if Instant::now() >= deadline {
            return procs_empty(dir);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

pub(crate) fn parse_supervisor_result(text: &str) -> Option<SupervisorOutcome> {
    let mut outcome = None;
    for line in complete_lines(text) {
        let mut parts = line.splitn(3, ' ');
        outcome = match (parts.next(), parts.next(), parts.next()) {
            (Some("result"), Some("ok"), Some(code)) => {
                code.parse().ok().map(SupervisorOutcome::Finished)
            }
            (Some("result"), Some("timeout"), None) => Some(SupervisorOutcome::Timeout),
            (Some("result"), Some("unkillable"), _) => Some(SupervisorOutcome::Unkillable),
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

pub(crate) fn parse_live_containment(text: &str) -> Option<LiveContainment> {
    let mut live = None;
    for line in complete_lines(text) {
        let mut parts = line.splitn(3, ' ');
        match (parts.next(), parts.next(), parts.next()) {
            (Some("mode"), Some("cgroup"), Some(path)) if path.starts_with('/') => {
                live = Some(LiveContainment::Cgroup(PathBuf::from(path)));
            }
            (Some("mode"), Some("pidns"), Some(pid)) => {
                if let Ok(pid) = pid.parse() {
                    live = Some(LiveContainment::PidNamespace(pid));
                }
            }
            (Some("mode"), Some("none"), _) => live = None,
            _ => {}
        }
    }
    live
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
        parse_live_containment, parse_supervisor_result, LiveContainment, SupervisorOutcome,
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
            parse_supervisor_result("result unkillable\n"),
            Some(SupervisorOutcome::Unkillable)
        ));
        assert!(parse_supervisor_result("result ok 127\n")
            .is_some_and(|outcome| { matches!(outcome, SupervisorOutcome::Finished(127)) }));
    }

    #[test]
    fn partial_trailing_line_is_ignored() {
        assert!(parse_supervisor_result("result timeout").is_none());
        assert!(matches!(
            parse_live_containment("mode pidns 42\nresult time"),
            Some(LiveContainment::PidNamespace(42))
        ));
    }
}
