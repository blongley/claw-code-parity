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
use std::cell::{Cell, RefCell};
use std::ffi::{CString, OsStr};
use std::io::{self, Read};
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;
use std::process::Command;
#[cfg(test)]
use std::sync::{Arc, Barrier};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec};
use rustix::io::Errno;
use rustix::process::{Signal, WaitId, WaitIdOptions, WaitIdStatus, WaitOptions, WaitStatus};

/// Status text for a process that survived teardown. Not an exit code.
pub(crate) const UNKILLABLE_STATUS: &str = "processes could not be killed";

const RELEASED: u8 = 1;
/// Exit codes from init before the ack byte. The parent maps each one to the
/// step that failed. They are not shell statuses.
const EXIT_INSTALL_FDS: i32 = 80;
const EXIT_CLOSE_FDS: i32 = 81;
const EXIT_PDEATHSIG: i32 = 82;
const EXIT_PARENT: i32 = 83;
const EXIT_ACK: i32 = 84;
const EMPTY_WAIT: Duration = Duration::from_secs(2);
/// Allowlisted child descriptors. Everything from 3 up is inherited and must
/// be closed, including another init's stdout writer.
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
    static FAIL_CLOSE: Cell<bool> = const { Cell::new(false) };
    static FORCE_FD_WALK: Cell<bool> = const { Cell::new(false) };
    static RELEASES: Cell<u32> = const { Cell::new(0) };
    static RELEASE_DELAY_MS: Cell<u64> = const { Cell::new(0) };
    static OVERLAP: RefCell<Option<Arc<Barrier>>> = const { RefCell::new(None) };
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
    Close,
    FdWalk,
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
pub(crate) fn inject_close_failure() -> FailureGuard {
    FAIL_CLOSE.with(|cell| cell.set(true));
    FailureGuard {
        kind: FailureKind::Close,
    }
}

/// Skip `close_range` and close descriptors one by one. The child still must
/// not allocate.
#[cfg(test)]
pub(crate) fn inject_fd_walk() -> FailureGuard {
    FORCE_FD_WALK.with(|cell| cell.set(true));
    FailureGuard {
        kind: FailureKind::FdWalk,
    }
}

#[cfg(test)]
#[must_use = "the release delay ends when this guard is dropped"]
pub(crate) struct ReleaseDelayGuard;

/// Sleep in the parent after id maps and before the release byte, so a short
/// deadline can pass after init is armed.
#[cfg(test)]
pub(crate) fn delay_release_by(delay: Duration) -> ReleaseDelayGuard {
    let millis = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX);
    RELEASE_DELAY_MS.with(|cell| cell.set(millis));
    ReleaseDelayGuard
}

#[cfg(test)]
impl Drop for ReleaseDelayGuard {
    fn drop(&mut self) {
        RELEASE_DELAY_MS.with(|cell| cell.set(0));
    }
}

#[cfg(test)]
#[must_use = "the overlap barrier ends when this guard is dropped"]
pub(crate) struct OverlapGuard;

/// Both launches wait after their pipes exist and before `clone3`, so each
/// init inherits the other's write ends.
#[cfg(test)]
pub(crate) fn overlap_at_clone(barrier: Arc<Barrier>) -> OverlapGuard {
    OVERLAP.with(|slot| *slot.borrow_mut() = Some(barrier));
    OverlapGuard
}

#[cfg(test)]
impl Drop for OverlapGuard {
    fn drop(&mut self) {
        OVERLAP.with(|slot| *slot.borrow_mut() = None);
    }
}

#[cfg(test)]
impl Drop for FailureGuard {
    fn drop(&mut self) {
        match self.kind {
            FailureKind::Namespace => FAIL_NAMESPACE.with(|cell| cell.set(false)),
            FailureKind::Arming => FAIL_ARMING.with(|cell| cell.set(false)),
            FailureKind::Close => FAIL_CLOSE.with(|cell| cell.set(false)),
            FailureKind::FdWalk => FORCE_FD_WALK.with(|cell| cell.set(false)),
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
    #[cfg(test)]
    wait_for_overlap();

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
                        detail: setup_failure_detail(code),
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
    #[cfg(test)]
    pause_before_release();
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
    close_fds: CloseFds,
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
            close_fds: configured_close_fds(),
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

fn configured_close_fds() -> CloseFds {
    #[cfg(test)]
    {
        if FAIL_CLOSE.with(Cell::get) {
            return CloseFds::Fail;
        }
        if FORCE_FD_WALK.with(Cell::get) {
            return CloseFds::Walk;
        }
    }
    CloseFds::Range
}

fn setup_failure_detail(code: i32) -> String {
    let step = match code {
        EXIT_INSTALL_FDS => "install allowlisted descriptors",
        EXIT_CLOSE_FDS => "close inherited descriptors",
        EXIT_PDEATHSIG => "arm parent-death signal",
        EXIT_PARENT => "confirm the original parent",
        EXIT_ACK => "write the readiness ack",
        _ => "complete containment setup",
    };
    format!("pid namespace init failed to {step} (exit {code})")
}

#[cfg(test)]
fn wait_for_overlap() {
    let barrier = OVERLAP.with(|slot| slot.borrow().clone());
    if let Some(barrier) = barrier {
        barrier.wait();
    }
}

#[cfg(test)]
fn pause_before_release() {
    let delay = RELEASE_DELAY_MS.with(Cell::get);
    if delay > 0 {
        thread::sleep(Duration::from_millis(delay));
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
    retain_allowlisted_fds(
        image.control,
        image.stdout_write,
        image.stderr_write,
        image.close_fds,
    );
    if image.fail_arming {
        raw_exit(EXIT_PDEATHSIG);
    }
    // Safety: `PR_SET_PDEATHSIG` takes the signal in the second argument.
    if unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) } != 0 {
        raw_exit(EXIT_PDEATHSIG);
    }
    // Inside the new pid namespace the creator is reported as 0. Any other
    // unexpected parent means this process was reparented before the death
    // signal was armed.
    if !parent_is_original(raw_getppid(), image.original_parent) {
        raw_exit(EXIT_PARENT);
    }
    if write_full(FD_CONTROL, &[RELEASED]).is_err() {
        raw_exit(EXIT_ACK);
    }
    let mut release = [0_u8; 1];
    if read_full(FD_CONTROL, &mut release).is_err() || release[0] != RELEASED {
        raw_exit(EXIT_ACK);
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

/// How init drops inherited descriptors. `Walk` skips `close_range` and reads
/// `/proc/self/fd`, which is also the `ENOSYS` fallback.
#[derive(Clone, Copy)]
enum CloseFds {
    Range,
    #[cfg(test)]
    Walk,
    #[cfg(test)]
    Fail,
}

enum RangeClose {
    Done,
    Fallback,
    Failed,
}

/// Dup the control socket and the command's stdout and stderr onto fds 0, 1,
/// and 2, then close every inherited descriptor from 3 up.
fn retain_allowlisted_fds(control: i32, stdout: i32, stderr: i32, close_fds: CloseFds) {
    let control = park_above_stdio(control);
    let stdout = park_above_stdio(stdout);
    let stderr = park_above_stdio(stderr);
    if !(dup_onto(control, FD_CONTROL)
        && dup_onto(stdout, FD_STDOUT)
        && dup_onto(stderr, FD_STDERR))
    {
        raw_exit(EXIT_INSTALL_FDS);
    }
    if !close_inherited_fds(close_fds) {
        raw_exit(EXIT_CLOSE_FDS);
    }
}

/// `close_range` arrived in Linux 5.9. Older kernels return `ENOSYS`. The
/// fallback lists the descriptors that are actually open. A walk up to the
/// soft limit misses fds already open above that ceiling, and an unlimited
/// limit would close empty slots until `i32::MAX` before parent-death is armed.
fn close_inherited_fds(mode: CloseFds) -> bool {
    match mode {
        #[cfg(test)]
        CloseFds::Fail => false,
        #[cfg(test)]
        CloseFds::Walk => close_open_fds_above_stdio(),
        CloseFds::Range => match close_range_above_stdio() {
            RangeClose::Done => true,
            RangeClose::Fallback => close_open_fds_above_stdio(),
            RangeClose::Failed => false,
        },
    }
}

fn close_range_above_stdio() -> RangeClose {
    // Safety: fds below 3 are the allowlist installed by the caller. Flag 0
    // closes the range instead of only marking it cloexec.
    let rc = unsafe { libc::close_range(3, libc::c_uint::MAX, 0) };
    let errno = if rc < 0 { last_errno() } else { 0 };
    range_close_from_result(rc, errno)
}

fn range_close_from_result(rc: i32, errno: i32) -> RangeClose {
    if rc == 0 {
        RangeClose::Done
    } else if errno == libc::ENOSYS {
        RangeClose::Fallback
    } else {
        RangeClose::Failed
    }
}

/// Close every open descriptor above stderr by listing `/proc/self/fd`.
///
/// Closing an entry can make the kernel skip later names, so each pass starts
/// at offset 0 and the scan repeats until a pass closes nothing. The directory
/// fd stays open until that idle pass. No heap allocation: names are parsed
/// out of a stack buffer. Every call is a raw syscall. The libc wrappers are
/// cancellation points, and this runs before `PR_SET_PDEATHSIG`.
fn close_open_fds_above_stdio() -> bool {
    let Some(dir) = raw_open_proc_fd_dir() else {
        return false;
    };
    let mut buf = [0_u8; 1024];
    loop {
        if !raw_rewind(dir) {
            let _ = raw_close_fd(dir);
            return false;
        }
        match close_one_proc_pass(dir, &mut buf) {
            Ok(true) => {}
            Ok(false) => return raw_close_fd(dir).is_ok(),
            Err(()) => {
                let _ = raw_close_fd(dir);
                return false;
            }
        }
    }
}

/// `true` when `/proc/self/fd` lists a descriptor above stderr other than the
/// directory being read. `None` when that directory cannot be read.
#[cfg(test)]
fn proc_has_fd_above_stdio() -> Option<bool> {
    let dir = raw_open_proc_fd_dir()?;
    let mut buf = [0_u8; 1024];
    if !raw_rewind(dir) {
        let _ = raw_close_fd(dir);
        return None;
    }
    let mut found = false;
    loop {
        let read = match raw_getdents(dir, &mut buf) {
            Ok(0) => break,
            Ok(count) => count,
            Err(_) => {
                let _ = raw_close_fd(dir);
                return None;
            }
        };
        let mut offset = 0;
        while offset < read {
            let Ok((next, fd)) = parse_dirent(&buf[..read], offset) else {
                let _ = raw_close_fd(dir);
                return None;
            };
            offset = next;
            if fd.is_some_and(|fd| fd > 2 && fd != dir) {
                found = true;
            }
        }
    }
    let _ = raw_close_fd(dir);
    Some(found)
}

fn close_one_proc_pass(dir: i32, buf: &mut [u8]) -> Result<bool, ()> {
    let mut closed_any = false;
    loop {
        let read = match raw_getdents(dir, buf) {
            Ok(0) => return Ok(closed_any),
            Ok(count) => count,
            // The cursor can be invalidated by a close earlier in this pass.
            // The caller restarts from offset 0.
            Err(_) if closed_any => return Ok(true),
            Err(_) => return Err(()),
        };
        let mut offset = 0;
        while offset < read {
            let (next, fd) = parse_dirent(&buf[..read], offset)?;
            offset = next;
            let Some(fd) = fd else {
                continue;
            };
            if fd <= 2 || fd == dir {
                continue;
            }
            raw_close_fd(fd)?;
            closed_any = true;
        }
    }
}

/// `linux_dirent64`: `d_ino` and `d_off` are 8 bytes each, then `d_reclen`,
/// `d_type`, and a NUL-terminated name. `.` and `..` are not descriptors.
fn parse_dirent(buf: &[u8], offset: usize) -> Result<(usize, Option<i32>), ()> {
    let header = offset.checked_add(19).ok_or(())?;
    if header > buf.len() {
        return Err(());
    }
    let reclen = usize::from(u16::from_ne_bytes([buf[offset + 16], buf[offset + 17]]));
    let next = offset.checked_add(reclen).ok_or(())?;
    if reclen < 19 || next > buf.len() {
        return Err(());
    }
    let name = &buf[header..next];
    let end = name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(name.len());
    Ok((next, parse_fd_name(&name[..end])))
}

fn parse_fd_name(name: &[u8]) -> Option<i32> {
    if name.is_empty() {
        return None;
    }
    let mut value = 0_i32;
    for &byte in name {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(i32::from(byte - b'0'))?;
    }
    Some(value)
}

fn raw_open_proc_fd_dir() -> Option<i32> {
    let path = c"/proc/self/fd";
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC;
    loop {
        // Safety: `path` is a NUL-terminated literal. `O_DIRECTORY` does not
        // create a file, so the mode is unused. `SYS_openat` is not a
        // cancellation point.
        let rc = unsafe {
            libc::syscall(
                libc::SYS_openat,
                libc::c_long::from(libc::AT_FDCWD),
                path.as_ptr(),
                libc::c_long::from(flags),
                libc::c_long::from(0_i32),
            )
        };
        if rc >= 0 {
            return i32::try_from(rc).ok();
        }
        if last_errno() != libc::EINTR {
            return None;
        }
    }
}

fn raw_getdents(dir: i32, buf: &mut [u8]) -> Result<usize, i32> {
    let count = libc::c_long::try_from(buf.len()).map_err(|_| libc::EINVAL)?;
    loop {
        // Safety: `buf` is a writable stack buffer of `count` bytes. The kernel
        // writes whole `linux_dirent64` records and does not retain the pointer.
        let rc = unsafe {
            libc::syscall(
                libc::SYS_getdents64,
                libc::c_long::from(dir),
                buf.as_mut_ptr(),
                count,
            )
        };
        if rc >= 0 {
            return usize::try_from(rc).map_err(|_| libc::EINVAL);
        }
        let err = last_errno();
        if err != libc::EINTR {
            return Err(err);
        }
    }
}

fn raw_rewind(dir: i32) -> bool {
    loop {
        // Safety: rewinding this directory fd to offset 0 restarts the scan.
        // `SYS_lseek` is not a cancellation point.
        let rc = unsafe {
            libc::syscall(
                libc::SYS_lseek,
                libc::c_long::from(dir),
                0_i64,
                libc::c_long::from(libc::SEEK_SET),
            )
        };
        if rc >= 0 {
            return true;
        }
        if last_errno() != libc::EINTR {
            return false;
        }
    }
}

fn raw_close_fd(fd: i32) -> Result<(), ()> {
    // Safety: `SYS_close` drops one descriptor and is not a cancellation point.
    // On `EINTR` the kernel may already have released it, so this does not
    // retry: a retry could close a number that was reused. The next pass
    // closes the number again if it is still open.
    let rc = unsafe { libc::syscall(libc::SYS_close, libc::c_long::from(fd)) };
    if rc == 0 {
        return Ok(());
    }
    let err = last_errno();
    if err == libc::EINTR || err == libc::EBADF {
        Ok(())
    } else {
        Err(())
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
        classify_reap, clone3, clone3_raw, close_inherited_fds, close_open_fds_above_stdio,
        dup_onto, exit_code_from_wait, exit_code_from_waitid, parent_is_original,
        proc_has_fd_above_stdio, range_close_from_result, raw_exit, reap_until_empty,
        resolve_program_in, retain_allowlisted_fds, setup_failure_detail, signal_pidfd, write_full,
        write_id_maps, CloseFds, RangeClose, Reap, EXIT_ACK, EXIT_CLOSE_FDS, EXIT_INSTALL_FDS,
        EXIT_PARENT, EXIT_PDEATHSIG,
    };
    use std::ffi::OsStr;
    use std::io::Read;
    use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn setup_failure_names_the_step_that_exited() {
        assert!(setup_failure_detail(EXIT_INSTALL_FDS).contains("install allowlisted descriptors"));
        assert!(setup_failure_detail(EXIT_CLOSE_FDS).contains("close inherited descriptors"));
        assert!(setup_failure_detail(EXIT_PDEATHSIG).contains("arm parent-death signal"));
        assert!(setup_failure_detail(EXIT_PARENT).contains("confirm the original parent"));
        assert!(setup_failure_detail(EXIT_ACK).contains("write the readiness ack"));
        let other = setup_failure_detail(3);
        assert!(other.contains("complete containment setup"));
        assert!(
            !other.contains("parent-death"),
            "an unknown exit was blamed on parent-death: {other}"
        );
    }

    #[test]
    fn close_range_enosys_selects_the_fd_walk() {
        assert!(matches!(
            range_close_from_result(0, libc::ENOSYS),
            RangeClose::Done
        ));
        assert!(matches!(
            range_close_from_result(-1, libc::ENOSYS),
            RangeClose::Fallback
        ));
        assert!(matches!(
            range_close_from_result(-1, libc::EPERM),
            RangeClose::Failed
        ));
        assert!(matches!(
            range_close_from_result(-1, libc::EINVAL),
            RangeClose::Failed
        ));
    }

    #[test]
    fn dup_onto_same_fd_clears_cloexec() {
        let (read, write) = std::io::pipe().expect("pipe");
        let fd = write.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        assert!(flags >= 0, "fcntl get");
        assert!(
            unsafe { libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) } >= 0,
            "fcntl set cloexec"
        );
        assert!(dup_onto(fd, fd), "same-fd dup should clear cloexec");
        let cleared = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        assert!(cleared >= 0);
        assert_eq!(cleared & libc::FD_CLOEXEC, 0);
        assert!(!dup_onto(-1, fd));
        assert!(!dup_onto(1_000_000, 1_000_000));
        drop(read);
    }

    #[test]
    fn same_fd_cloexec_branch_keeps_stdout_across_exec() {
        let (mut read, write) = std::io::pipe().expect("pipe");
        let pid = clone3(0, None, None).expect("fork");
        if pid == 0 {
            keep_cloexec_stdout(write.as_raw_fd());
        }
        drop(write);
        let (_, status) = rustix::process::waitpid(
            rustix::process::Pid::from_raw(pid),
            rustix::process::WaitOptions::empty(),
        )
        .expect("wait")
        .expect("child status");
        let mut got = String::new();
        read.read_to_string(&mut got).expect("stdout");
        assert_eq!(exit_code_from_wait(status), 0, "echo exited {got:?}");
        assert_eq!(got, "kept\n");
    }

    fn keep_cloexec_stdout(fd: i32) -> ! {
        unsafe {
            if libc::dup2(fd, 1) < 0 {
                raw_exit(2);
            }
            if fd != 1 {
                libc::close(fd);
            }
            if libc::fcntl(1, libc::F_SETFD, libc::FD_CLOEXEC) < 0 {
                raw_exit(3);
            }
        }
        if !dup_onto(1, 1) {
            raw_exit(4);
        }
        unsafe {
            let program_arg = c"echo";
            let message = c"kept";
            let argv = [program_arg.as_ptr(), message.as_ptr(), std::ptr::null()];
            let envp = [std::ptr::null::<libc::c_char>()];
            libc::execve(c"/bin/echo".as_ptr(), argv.as_ptr(), envp.as_ptr());
        }
        raw_exit(5);
    }

    #[test]
    fn control_and_pipe_fds_on_stdio_survive_the_allowlist() {
        let (mut stdout_read, stdout_write) = std::io::pipe().expect("stdout");
        let (mut stderr_read, stderr_write) = std::io::pipe().expect("stderr");
        let (mut parent_control, child_control) = super::control_socket().expect("control");
        let pid = clone3(0, None, None).expect("fork");
        if pid == 0 {
            for fd in [
                stdout_read.as_raw_fd(),
                stderr_read.as_raw_fd(),
                parent_control.as_raw_fd(),
            ] {
                unsafe { libc::close(fd) };
            }
            scramble_stdio_and_retain(
                child_control.as_raw_fd(),
                stdout_write.as_raw_fd(),
                stderr_write.as_raw_fd(),
            );
        }
        drop(stdout_write);
        drop(stderr_write);
        drop(child_control);
        let (_, status) = rustix::process::waitpid(
            rustix::process::Pid::from_raw(pid),
            rustix::process::WaitOptions::empty(),
        )
        .expect("wait")
        .expect("child status");
        let mut ack = [0_u8; 1];
        std::io::Read::read_exact(&mut parent_control, &mut ack).expect("ack");
        let mut out = [0_u8; 3];
        std::io::Read::read_exact(&mut stdout_read, &mut out).expect("stdout");
        let mut err = [0_u8; 3];
        std::io::Read::read_exact(&mut stderr_read, &mut err).expect("stderr");
        assert_eq!(exit_code_from_wait(status), 0, "allowlist install failed");
        assert_eq!(ack, [1]);
        assert_eq!(&out, b"out");
        assert_eq!(&err, b"err");
    }

    fn scramble_stdio_and_retain(control: i32, stdout: i32, stderr: i32) -> ! {
        unsafe {
            // Park the real ends, then drop stdio so the next dups land on 0, 1,
            // and 2 in the wrong order. `retain_allowlisted_fds` has to move
            // them above stdio before installing the allowlist.
            let hold_control = libc::fcntl(control, libc::F_DUPFD, 3);
            let hold_stdout = libc::fcntl(stdout, libc::F_DUPFD, 3);
            let hold_stderr = libc::fcntl(stderr, libc::F_DUPFD, 3);
            if hold_control < 0 || hold_stdout < 0 || hold_stderr < 0 {
                raw_exit(2);
            }
            libc::close(control);
            libc::close(stdout);
            libc::close(stderr);
            libc::close(0);
            libc::close(1);
            libc::close(2);
            if libc::dup2(hold_stdout, 0) < 0
                || libc::dup2(hold_control, 1) < 0
                || libc::dup2(hold_stderr, 2) < 0
            {
                raw_exit(3);
            }
            libc::close(hold_stdout);
            libc::close(hold_control);
            libc::close(hold_stderr);
        }
        // control is 1, stdout is 0, stderr is 2.
        retain_allowlisted_fds(1, 0, 2, CloseFds::Range);
        if write_full(0, &[1]).is_err()
            || write_full(1, b"out").is_err()
            || write_full(2, b"err").is_err()
        {
            raw_exit(4);
        }
        if proc_has_fd_above_stdio() != Some(false) {
            raw_exit(5);
        }
        raw_exit(0);
    }

    #[test]
    fn proc_scan_closes_fds_open_above_a_lowered_soft_limit() {
        let pid = clone3(0, None, None).expect("fork");
        if pid == 0 {
            close_fds_above_lowered_limit();
        }
        let (_, status) = rustix::process::waitpid(
            rustix::process::Pid::from_raw(pid),
            rustix::process::WaitOptions::empty(),
        )
        .expect("wait")
        .expect("child status");
        assert_eq!(
            exit_code_from_wait(status),
            0,
            "an fd above the lowered soft limit stayed open ({})",
            exit_code_from_wait(status)
        );
    }

    fn close_fds_above_lowered_limit() -> ! {
        unsafe {
            let mut limit = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if libc::getrlimit(libc::RLIMIT_NOFILE, &raw mut limit) != 0 {
                raw_exit(2);
            }
            let ceiling = i32::try_from(limit.rlim_cur).unwrap_or(i32::MAX);
            if ceiling < 8 {
                raw_exit(3);
            }
            let high = libc::fcntl(0, libc::F_DUPFD, 80.min(ceiling - 1));
            if high < 3 {
                raw_exit(4);
            }
            // The descriptor is already open. Lowering the soft ceiling to its
            // number makes it illegal to open again, but it is still open.
            let Ok(lowered) = libc::rlim_t::try_from(high) else {
                raw_exit(5);
            };
            limit.rlim_cur = lowered;
            if libc::setrlimit(libc::RLIMIT_NOFILE, &raw const limit) != 0 {
                raw_exit(5);
            }
            if !close_open_fds_above_stdio() {
                raw_exit(6);
            }
            let still_open = libc::fcntl(high, libc::F_GETFD) >= 0;
            let stdio_open = libc::fcntl(0, libc::F_GETFD) >= 0;
            raw_exit(if !still_open && stdio_open { 0 } else { 7 });
        }
    }

    #[test]
    fn proc_scan_finishes_quickly_at_an_extreme_soft_limit() {
        let pid = clone3(0, None, None).expect("fork");
        if pid == 0 {
            scan_at_extreme_limit();
        }
        let Some(pid) = rustix::process::Pid::from_raw(pid) else {
            panic!("fork returned a pid that is not a process id");
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        loop {
            match rustix::process::waitpid(Some(pid), rustix::process::WaitOptions::NOHANG) {
                Ok(Some((_, status))) => {
                    assert_eq!(
                        exit_code_from_wait(status),
                        0,
                        "extreme-limit scan failed ({})",
                        exit_code_from_wait(status)
                    );
                    return;
                }
                Ok(None) if std::time::Instant::now() >= deadline => {
                    let _ = rustix::process::kill_process(pid, rustix::process::Signal::KILL);
                    let _ =
                        rustix::process::waitpid(Some(pid), rustix::process::WaitOptions::empty());
                    panic!("descriptor scan was still running after 1s at the maximum soft limit");
                }
                Ok(None) | Err(rustix::io::Errno::INTR) => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(err) => panic!("wait: {err}"),
            }
        }
    }

    fn scan_at_extreme_limit() -> ! {
        unsafe {
            let mut limit = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if libc::getrlimit(libc::RLIMIT_NOFILE, &raw mut limit) != 0 {
                raw_exit(2);
            }
            // An unlimited ceiling is the case that used to walk `i32::MAX`.
            // Unprivileged processes cannot raise the hard limit, so a rejected
            // request falls back to that hard limit, which is still far above
            // the descriptors this process has open.
            let mut unlimited = limit;
            unlimited.rlim_cur = libc::RLIM_INFINITY;
            if libc::setrlimit(libc::RLIMIT_NOFILE, &raw const unlimited) != 0 {
                limit.rlim_cur = limit.rlim_max;
                if libc::setrlimit(libc::RLIMIT_NOFILE, &raw const limit) != 0 {
                    raw_exit(3);
                }
            }
            let high = libc::fcntl(0, libc::F_DUPFD, 3);
            if high < 3 {
                raw_exit(4);
            }
            let mut start = libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            if libc::clock_gettime(libc::CLOCK_MONOTONIC, &raw mut start) != 0 {
                raw_exit(5);
            }
            if !close_open_fds_above_stdio() {
                raw_exit(6);
            }
            let mut end = libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            if libc::clock_gettime(libc::CLOCK_MONOTONIC, &raw mut end) != 0 {
                raw_exit(7);
            }
            let elapsed_ns =
                (end.tv_sec - start.tv_sec) * 1_000_000_000 + (end.tv_nsec - start.tv_nsec);
            if elapsed_ns > 200_000_000 || libc::fcntl(high, libc::F_GETFD) >= 0 {
                raw_exit(8);
            }
            raw_exit(0);
        }
    }

    #[test]
    fn close_range_failure_does_not_report_success() {
        assert!(!close_inherited_fds(CloseFds::Fail));
    }

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
