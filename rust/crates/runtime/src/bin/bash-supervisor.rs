//! Adopts detached descendants of one shell command.
//!
//! Timed commands need a process that outlives `setsid`. Doing that with
//! `python3 -c` from the workspace puts the workspace on Python's import
//! path, so local code runs outside the sandbox. This program enables
//! `PR_SET_CHILD_SUBREAPER` itself, enforces the caller's deadline, and exits
//! only after `waitpid` returns `ECHILD`.

use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Exit status the parent treats as a timeout when it arrives at the deadline.
const TIMEOUT_EXIT: i32 = 124;

fn main() {
    let code = supervise().unwrap_or(127);
    std::process::exit(code);
}

#[cfg(not(target_os = "linux"))]
fn supervise() -> io::Result<i32> {
    Err(io::Error::other(
        "the bash supervisor requires a Linux subreaper",
    ))
}

#[cfg(target_os = "linux")]
fn supervise() -> io::Result<i32> {
    let (timeout_ms, line) = parse_args()?;
    // `Some(Pid::INIT)` is the prctl boolean, not a request to adopt pid 1.
    rustix::process::set_child_subreaper(Some(rustix::process::Pid::INIT))?;

    // Best-effort: a child cgroup lets `cgroup.kill` reach descendants that
    // fork during teardown. The supervisor stays in its original cgroup so
    // that kill does not hit this process. `ECHILD` is still the proof.
    let cgroup = ShellCgroup::create();
    let shell = {
        let child = Command::new("/bin/sh")
            .arg("-c")
            .arg(line)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;
        let shell = rustix::process::Pid::from_child(&child);
        if let Some(cgroup) = &cgroup {
            cgroup.adopt(child.id());
        }
        // `Child`'s destructor does not wait or signal. `wait(-1)` below is
        // the only reap, so dropping the handle must not race with it.
        shell
    };

    let deadline = Instant::now() + duration_from_millis(timeout_ms);
    let mut shell_status = None;
    loop {
        if reap(shell, &mut shell_status)? == Reap::NoChildren {
            if let Some(status) = shell_status {
                return Ok(status);
            }
            return Err(io::Error::other("shell exited without a status"));
        }
        if Instant::now() >= deadline {
            kill_until_echild(cgroup.as_ref())?;
            return Ok(TIMEOUT_EXIT);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(target_os = "linux")]
fn parse_args() -> io::Result<(u64, String)> {
    let mut args = std::env::args();
    let _argv0 = args.next();
    let Some(timeout) = args.next() else {
        return Err(io::Error::other("bash supervisor requires a timeout"));
    };
    let Some(line) = args.next() else {
        return Err(io::Error::other("bash supervisor requires a command"));
    };
    let timeout_ms = timeout.parse::<u64>().map_err(|_| {
        io::Error::other("bash supervisor timeout must be a number of milliseconds")
    })?;
    Ok((timeout_ms, line))
}

#[cfg(target_os = "linux")]
fn duration_from_millis(ms: u64) -> Duration {
    Duration::from_millis(ms.min(u64::MAX / 1_000_000))
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reap {
    ChildrenRemain,
    NoChildren,
}

#[cfg(target_os = "linux")]
fn reap(shell: rustix::process::Pid, shell_status: &mut Option<i32>) -> io::Result<Reap> {
    loop {
        match rustix::process::wait(rustix::process::WaitOptions::NOHANG) {
            Ok(Some((pid, status))) => {
                if pid == shell && shell_status.is_none() {
                    if let Some(code) = finished_code(status) {
                        *shell_status = Some(code);
                    }
                }
            }
            Ok(None) => return Ok(Reap::ChildrenRemain),
            Err(rustix::io::Errno::CHILD) => return Ok(Reap::NoChildren),
            Err(rustix::io::Errno::INTR) => {}
            Err(err) => return Err(err.into()),
        }
    }
}

#[cfg(target_os = "linux")]
fn finished_code(status: rustix::process::WaitStatus) -> Option<i32> {
    if let Some(code) = status.exit_status() {
        return Some(code);
    }
    status
        .terminating_signal()
        .map(|signal| 128_i32.saturating_add(signal))
}

/// Signal every adopted child, then reap, until `waitpid` returns `ECHILD`.
///
/// A single `/proc` snapshot misses a process that forks after the scan.
/// `ECHILD` is the proof that nothing is left, including zombies.
#[cfg(target_os = "linux")]
fn kill_until_echild(cgroup: Option<&ShellCgroup>) -> io::Result<()> {
    let give_up = Instant::now() + Duration::from_millis(750);
    loop {
        signal_adopted_children(cgroup);
        match rustix::process::wait(rustix::process::WaitOptions::NOHANG) {
            Ok(None) => {
                if Instant::now() >= give_up {
                    return Err(io::Error::other(
                        "descendants were still alive after SIGKILL",
                    ));
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(rustix::io::Errno::CHILD) => return Ok(()),
            Ok(Some(_)) | Err(rustix::io::Errno::INTR) => {}
            Err(err) => return Err(err.into()),
        }
    }
}

#[cfg(target_os = "linux")]
fn signal_adopted_children(cgroup: Option<&ShellCgroup>) {
    if let Some(cgroup) = cgroup {
        cgroup.kill_all();
    }
    let me = std::process::id();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return;
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        if pid == me {
            continue;
        }
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        if stat_parent(&stat) != Some(me) {
            continue;
        }
        let Some(raw) = i32::try_from(pid)
            .ok()
            .and_then(rustix::process::Pid::from_raw)
        else {
            continue;
        };
        let _ = rustix::process::kill_process(raw, rustix::process::Signal::KILL);
    }
}

#[cfg(target_os = "linux")]
fn stat_parent(stat: &str) -> Option<u32> {
    let (_, after_comm) = stat.rsplit_once(')')?;
    let mut fields = after_comm.split_whitespace();
    let _state = fields.next()?;
    fields.next()?.parse().ok()
}

/// A cgroup that contains the shell and not the supervisor.
#[cfg(target_os = "linux")]
struct ShellCgroup {
    dir: PathBuf,
}

#[cfg(target_os = "linux")]
impl ShellCgroup {
    fn create() -> Option<Self> {
        let listing = std::fs::read_to_string("/proc/self/cgroup").ok()?;
        let relative = listing.lines().find_map(|line| line.strip_prefix("0::"))?;
        let parent = PathBuf::from("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
        let dir = parent.join(format!("bash-supervisor-{}", std::process::id()));
        if std::fs::create_dir(&dir).is_err() {
            return None;
        }
        if !dir.join("cgroup.kill").is_file() {
            let _ = std::fs::remove_dir(&dir);
            return None;
        }
        Some(Self { dir })
    }

    fn adopt(&self, pid: u32) {
        let _ = std::fs::write(self.dir.join("cgroup.procs"), pid.to_string());
    }

    fn kill_all(&self) {
        let kill = self.dir.join("cgroup.kill");
        if kill.is_file() {
            let _ = std::fs::write(kill, "1");
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for ShellCgroup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir(&self.dir);
    }
}
