//! Adopts detached descendants of one shell command.
//!
//! Timed commands need a process that outlives `setsid`. Doing that with
//! `python3 -c` from the workspace puts the workspace on Python's import
//! path, so local code runs outside the sandbox. This program enables
//! `PR_SET_CHILD_SUBREAPER` itself and exits only after the shell and every
//! reparented descendant have been reaped.

use std::io;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

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
    let mut args = std::env::args();
    let _argv0 = args.next();
    let Some(line) = args.next() else {
        return Err(io::Error::other("bash supervisor requires a command"));
    };
    // `Some(Pid::INIT)` is the prctl boolean, not a request to adopt pid 1.
    rustix::process::set_child_subreaper(Some(rustix::process::Pid::INIT))?;

    let child = Command::new("/bin/sh")
        .arg("-c")
        .arg(line)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    let shell = rustix::process::Pid::from_child(&child);
    let root = std::process::id();
    let mut shell_status = None;
    loop {
        reap(shell, &mut shell_status)?;
        if let Some(status) = shell_status {
            if !has_live_descendant(root) {
                reap(shell, &mut shell_status)?;
                return Ok(shell_status.unwrap_or(status));
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(target_os = "linux")]
fn reap(shell: rustix::process::Pid, shell_status: &mut Option<i32>) -> io::Result<()> {
    loop {
        match rustix::process::wait(rustix::process::WaitOptions::NOHANG) {
            Ok(Some((pid, status))) => {
                if pid == shell && shell_status.is_none() {
                    if let Some(code) = finished_code(status) {
                        *shell_status = Some(code);
                    }
                }
            }
            Ok(None) | Err(rustix::io::Errno::CHILD) => return Ok(()),
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

/// A zombie still has a pid until it is reaped. It is not running the command.
#[cfg(target_os = "linux")]
fn has_live_descendant(root: u32) -> bool {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return true;
    };
    let mut parents = std::collections::HashMap::<u32, u32>::new();
    let mut alive = Vec::new();
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if !name.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        if pid == root {
            continue;
        }
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        let Some((state, parent)) = stat_state_and_parent(&stat) else {
            continue;
        };
        parents.insert(pid, parent);
        if state != "Z" {
            alive.push(pid);
        }
    }
    alive
        .into_iter()
        .any(|pid| descends_from(pid, root, &parents))
}

#[cfg(target_os = "linux")]
fn descends_from(mut pid: u32, root: u32, parents: &std::collections::HashMap<u32, u32>) -> bool {
    let mut seen = std::collections::HashSet::new();
    while seen.insert(pid) {
        let Some(parent) = parents.get(&pid).copied() else {
            return false;
        };
        if parent == root {
            return true;
        }
        if parent == 0 || parent == pid {
            return false;
        }
        pid = parent;
    }
    false
}

#[cfg(target_os = "linux")]
fn stat_state_and_parent(stat: &str) -> Option<(&str, u32)> {
    let (_, after_comm) = stat.rsplit_once(')')?;
    let mut fields = after_comm.split_whitespace();
    let state = fields.next()?;
    let parent = fields.next()?.parse().ok()?;
    Some((state, parent))
}
