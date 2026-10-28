#[cfg(target_os = "linux")]
use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::OsStr;
use std::io::{self, Read};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::OnceLock;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::sandbox::{
    build_linux_sandbox_command, resolve_sandbox_status_for_request, FilesystemIsolationMode,
    SandboxConfig, SandboxStatus,
};
use crate::ConfigLoader;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BashCommandInput {
    pub command: String,
    pub timeout: Option<u64>,
    pub description: Option<String>,
    #[serde(rename = "run_in_background")]
    pub run_in_background: Option<bool>,
    #[serde(rename = "dangerouslyDisableSandbox")]
    pub dangerously_disable_sandbox: Option<bool>,
    #[serde(rename = "namespaceRestrictions")]
    pub namespace_restrictions: Option<bool>,
    #[serde(rename = "isolateNetwork")]
    pub isolate_network: Option<bool>,
    #[serde(rename = "filesystemMode")]
    pub filesystem_mode: Option<FilesystemIsolationMode>,
    #[serde(rename = "allowedMounts")]
    pub allowed_mounts: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BashCommandOutput {
    pub stdout: String,
    pub stderr: String,
    #[serde(rename = "rawOutputPath")]
    pub raw_output_path: Option<String>,
    pub interrupted: bool,
    #[serde(rename = "isImage")]
    pub is_image: Option<bool>,
    #[serde(rename = "backgroundTaskId")]
    pub background_task_id: Option<String>,
    #[serde(rename = "backgroundedByUser")]
    pub backgrounded_by_user: Option<bool>,
    #[serde(rename = "assistantAutoBackgrounded")]
    pub assistant_auto_backgrounded: Option<bool>,
    #[serde(rename = "dangerouslyDisableSandbox")]
    pub dangerously_disable_sandbox: Option<bool>,
    #[serde(rename = "returnCodeInterpretation")]
    pub return_code_interpretation: Option<String>,
    #[serde(rename = "noOutputExpected")]
    pub no_output_expected: Option<bool>,
    #[serde(rename = "structuredContent")]
    pub structured_content: Option<Vec<serde_json::Value>>,
    #[serde(rename = "persistedOutputPath")]
    pub persisted_output_path: Option<String>,
    #[serde(rename = "persistedOutputSize")]
    pub persisted_output_size: Option<u64>,
    #[serde(rename = "sandboxStatus")]
    pub sandbox_status: Option<SandboxStatus>,
}

pub fn execute_bash(input: BashCommandInput) -> io::Result<BashCommandOutput> {
    let cwd = env::current_dir()?;
    let sandbox_status = sandbox_status_for_input(&input, &cwd);
    // A failed probe marks network isolation inactive and used to fall through
    // to a direct shell. That runs the command on the host network. Refuse
    // instead of dropping a boundary the caller asked for.
    if let Some(error) = refused_network_isolation(&sandbox_status) {
        return Err(error);
    }
    let BashCommandInput {
        command,
        timeout,
        run_in_background,
        dangerously_disable_sandbox,
        ..
    } = input;

    if run_in_background.unwrap_or(false) {
        let mut child = prepare_command(&command, &cwd, &sandbox_status, false);
        let child = child
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        return Ok(BashCommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            raw_output_path: None,
            interrupted: false,
            is_image: None,
            background_task_id: Some(child.id().to_string()),
            backgrounded_by_user: Some(false),
            assistant_auto_backgrounded: Some(false),
            dangerously_disable_sandbox,
            return_code_interpretation: None,
            no_output_expected: Some(true),
            structured_content: None,
            persisted_output_path: None,
            persisted_output_size: None,
            sandbox_status: Some(sandbox_status),
        });
    }

    let mut prepared = prepare_command(&command, &cwd, &sandbox_status, true);
    if let Some(timeout_ms) = timeout {
        execute_with_timeout(
            prepared,
            timeout_ms,
            dangerously_disable_sandbox,
            sandbox_status,
        )
    } else {
        let output = prepared.output()?;
        Ok(completed_output(
            &output,
            dangerously_disable_sandbox,
            sandbox_status,
        ))
    }
}

fn execute_with_timeout(
    mut command: Command,
    timeout_ms: u64,
    dangerously_disable_sandbox: Option<bool>,
    sandbox_status: SandboxStatus,
) -> io::Result<BashCommandOutput> {
    // Detached descendants (`setsid`, a new process group) leave the shell's
    // process group. A pid-namespace supervisor keeps them in one tree and
    // reaps them. Where that namespace is unavailable, a child subreaper
    // adopts the orphans so the deadline can still kill them. Stdout and
    // stderr are drained concurrently so a full pipe cannot stall the command.
    let supervision = supervision_mode();
    if let Some(supervised) = supervised_command(&command, supervision) {
        command = supervised;
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        command.process_group(0);
    }
    let mut child = command.spawn()?;
    let stdout_reader = spawn_reader(child.stdout.take());
    let stderr_reader = spawn_reader(child.stderr.take());
    let timeout = Duration::from_millis(timeout_ms);
    let wait_result = match supervision {
        Supervision::ProcessGroup => wait_for_process_group(&mut child, timeout),
        Supervision::PidNamespace | Supervision::Subreaper => wait_for_child(&mut child, timeout),
    };
    if wait_result.is_err() {
        terminate_child_tree(&mut child, false);
    }
    let stdout = join_reader(stdout_reader);
    let stderr = join_reader(stderr_reader);
    let status = wait_result?;
    let stdout = stdout?;
    let stderr = stderr?;

    match status {
        Some(status) => Ok(completed_output(
            &Output {
                status,
                stdout,
                stderr,
            },
            dangerously_disable_sandbox,
            sandbox_status,
        )),
        None => Ok(timed_out_output(
            &stdout,
            &stderr,
            timeout_ms,
            dangerously_disable_sandbox,
            sandbox_status,
        )),
    }
}

fn spawn_reader(pipe: Option<impl Read + Send + 'static>) -> JoinHandle<io::Result<Vec<u8>>> {
    thread::spawn(move || match pipe {
        Some(mut pipe) => {
            let mut buffer = Vec::new();
            pipe.read_to_end(&mut buffer)?;
            Ok(buffer)
        }
        None => Ok(Vec::new()),
    })
}

fn join_reader(handle: JoinHandle<io::Result<Vec<u8>>>) -> io::Result<Vec<u8>> {
    match handle.join() {
        Ok(result) => result,
        Err(_) => Err(io::Error::other(
            "command output reader stopped unexpectedly",
        )),
    }
}

/// Supervisor script run as pid 1 inside `unshare --pid`. It records the
/// shell status, reaps every descendant (including `setsid` orphans and
/// zombies), and exits only when nothing else in the namespace is alive.
const PID_NAMESPACE_SUPERVISOR: &str = r#"
target=$1
sh -c "$target" &
child=$!
status=0
wait "$child" || status=$?
alive() {
  for entry in /proc/[0-9]*; do
    pid=${entry##*/}
    if [ "$pid" = "1" ]; then
      continue
    fi
    stat=$(cat "$entry/stat" 2>/dev/null) || continue
    rest=${stat#*)}
    rest=${rest# }
    state=${rest%% *}
    if [ "$state" != "Z" ]; then
      return 0
    fi
  done
  return 1
}
while alive; do
  wait || true
  sleep 0.05
done
wait || true
exit "$status"
"#;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Supervision {
    PidNamespace,
    Subreaper,
    ProcessGroup,
}

fn supervision_mode() -> Supervision {
    // A child subreaper does not need a user namespace. Hosts that block
    // unprivileged `unshare --user` still adopt `setsid` orphans this way.
    // The pid namespace is the fallback when Python cannot call prctl.
    if subreaper_supervisor_available() {
        Supervision::Subreaper
    } else if pid_namespace_supervisor_available() {
        Supervision::PidNamespace
    } else {
        Supervision::ProcessGroup
    }
}

fn supervised_command(command: &Command, supervision: Supervision) -> Option<Command> {
    match supervision {
        Supervision::PidNamespace => Some(command_in_pid_namespace(command)),
        Supervision::Subreaper => Some(command_in_subreaper(command)),
        Supervision::ProcessGroup => None,
    }
}

fn pid_namespace_supervisor_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("unshare")
            .args([
                "--user",
                "--map-root-user",
                "--pid",
                "--fork",
                "--mount-proc",
                "--kill-child",
                "--",
                "sh",
                "-c",
                "printf ok",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .is_ok_and(|output| output.status.success() && output.stdout == b"ok")
    })
}

/// `PR_SET_CHILD_SUBREAPER`. Orphans of the command are reparented here instead
/// of to init, including processes that called `setsid`.
const SUBREAPER_SUPERVISOR: &str = r#"
import ctypes, os, signal, subprocess, sys, time
PR_SET_CHILD_SUBREAPER = 36
libc = ctypes.CDLL(None, use_errno=True)
if libc.prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) != 0:
    raise SystemExit(127)
target = sys.argv[1]
child = subprocess.Popen(["sh", "-c", target])
shell_status = None

def reap():
    global shell_status
    while True:
        try:
            pid, status = os.waitpid(-1, os.WNOHANG)
        except ChildProcessError:
            return
        if pid == 0:
            return
        if pid == child.pid and shell_status is None:
            shell_status = os.waitstatus_to_exitcode(status)

def read_stat(pid):
    try:
        with open(f"/proc/{pid}/stat", "r", encoding="utf-8", errors="replace") as handle:
            data = handle.read()
    except OSError:
        return None
    marker = data.rfind(")")
    if marker < 0:
        return None
    fields = data[marker + 1:].split()
    if len(fields) < 3:
        return None
    return fields[0], int(fields[1])

def live_descendants():
    root = os.getpid()
    info = {}
    for name in os.listdir("/proc"):
        if not name.isdigit():
            continue
        parsed = read_stat(int(name))
        if parsed is not None:
            info[int(name)] = parsed
    live = []
    for pid, (state, _ppid) in info.items():
        if state == "Z" or pid == root:
            continue
        seen = set()
        current = pid
        while current not in seen:
            seen.add(current)
            parent = info.get(current, (None, None))[1]
            if parent in (None, 0, current):
                break
            if parent == root:
                live.append(pid)
                break
            current = parent
    return live

def kill_descendants():
    for pid in live_descendants():
        try:
            os.kill(pid, signal.SIGKILL)
        except OSError:
            pass
    try:
        os.kill(child.pid, signal.SIGKILL)
    except OSError:
        pass
    reap()

def on_term(_signum, _frame):
    kill_descendants()
    raise SystemExit(128 + signal.SIGTERM)

signal.signal(signal.SIGTERM, on_term)
while True:
    reap()
    if shell_status is not None and not live_descendants():
        reap()
        raise SystemExit(shell_status if 0 <= shell_status < 256 else 128)
    time.sleep(0.01)
"#;

fn subreaper_supervisor_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("python3")
            .args([
                "-c",
                "import ctypes,sys\nlibc=ctypes.CDLL(None, use_errno=True)\nsys.exit(0 if libc.prctl(36, 1, 0, 0, 0) == 0 else 1)",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    })
}

fn command_in_pid_namespace(command: &Command) -> Command {
    let line = command_line(command);
    let mut supervised = command_with_context("unshare", command);
    supervised.args([
        "--user",
        "--map-root-user",
        "--pid",
        "--fork",
        "--mount-proc",
        "--kill-child",
        "--",
        "sh",
        "-c",
        PID_NAMESPACE_SUPERVISOR,
        "sh",
        &line,
    ]);
    supervised
}

fn command_in_subreaper(command: &Command) -> Command {
    let line = command_line(command);
    let mut supervised = command_with_context("python3", command);
    supervised.args(["-c", SUBREAPER_SUPERVISOR, &line]);
    supervised
}

fn command_with_context(program: &str, source: &Command) -> Command {
    let current_dir = source.get_current_dir().map(PathBuf::from);
    let envs = source
        .get_envs()
        .map(|(key, value)| (key.to_os_string(), value.map(OsStr::to_os_string)))
        .collect::<Vec<_>>();
    let mut command = Command::new(program);
    if let Some(dir) = current_dir {
        command.current_dir(dir);
    }
    for (key, value) in envs {
        match value {
            Some(value) => {
                command.env(key, value);
            }
            None => {
                command.env_remove(key);
            }
        }
    }
    command
}

fn command_line(command: &Command) -> String {
    let mut parts = Vec::new();
    parts.push(shell_quote(&command.get_program().to_string_lossy()));
    for arg in command.get_args() {
        parts.push(shell_quote(&arg.to_string_lossy()));
    }
    parts.join(" ")
}

fn shell_quote(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', "'\\''"))
}

fn wait_for_child(child: &mut Child, timeout: Duration) -> io::Result<Option<ExitStatus>> {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if started.elapsed() >= timeout {
            terminate_child_tree(child, false);
            return Ok(None);
        }
        let remaining = timeout.saturating_sub(started.elapsed());
        thread::sleep(Duration::from_millis(10).min(remaining));
    }
}

/// Waits until `child` and every live member of its process group have exited,
/// or until `timeout` elapses. Zombies are not members: a finished child that
/// has not been reaped yet must not turn a completed command into a timeout.
/// `Some` is the shell status after the group is empty. `None` means the
/// deadline fired and the group was killed.
fn wait_for_process_group(child: &mut Child, timeout: Duration) -> io::Result<Option<ExitStatus>> {
    let started = Instant::now();
    let group_id = child.id();
    let mut exit_status = None;
    loop {
        if exit_status.is_none() {
            exit_status = child.try_wait()?;
        }
        if exit_status.is_some() && !process_group_has_members(group_id) {
            return Ok(exit_status);
        }
        if started.elapsed() >= timeout {
            terminate_child_tree(child, exit_status.is_some());
            return Ok(None);
        }
        let remaining = timeout.saturating_sub(started.elapsed());
        thread::sleep(Duration::from_millis(10).min(remaining));
    }
}

fn terminate_child_tree(child: &mut Child, shell_reaped: bool) {
    #[cfg(target_os = "linux")]
    kill_linux_descendants(child.id());
    terminate_process_group(child, shell_reaped);
}

#[cfg(target_os = "linux")]
fn kill_linux_descendants(root: u32) {
    for _ in 0..2 {
        let pids = descendant_pids(root);
        if pids.is_empty() {
            break;
        }
        for pid in pids {
            signal_kill(pid);
        }
    }
}

#[cfg(target_os = "linux")]
fn descendant_pids(root: u32) -> Vec<u32> {
    let mut parents = HashMap::<u32, u32>::new();
    let mut states = HashMap::<u32, String>::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
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
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        let (Some(state), Some(parent)) = (process_state(&stat), parent_pid(&stat)) else {
            continue;
        };
        parents.insert(pid, parent);
        states.insert(pid, state.to_string());
    }

    let mut live = Vec::new();
    for (pid, state) in &states {
        if state == "Z" || *pid == root {
            continue;
        }
        let mut current = *pid;
        let mut seen = HashSet::new();
        while seen.insert(current) {
            let Some(parent) = parents.get(&current).copied() else {
                break;
            };
            if parent == root {
                live.push(*pid);
                break;
            }
            if parent == 0 || parent == current {
                break;
            }
            current = parent;
        }
    }
    live
}

fn terminate_process_group(child: &mut Child, shell_reaped: bool) {
    #[cfg(unix)]
    signal_process_group(child.id());
    if !shell_reaped {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(unix)]
fn signal_process_group(group_id: u32) {
    let group = format!("-{group_id}");
    signal_kill_arg(&group);
}

#[cfg(target_os = "linux")]
fn signal_kill(pid: u32) {
    signal_kill_arg(&pid.to_string());
}

#[cfg(unix)]
fn signal_kill_arg(target: &str) {
    let _ = Command::new("kill")
        .args(["-KILL", "--", target])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(target_os = "linux")]
fn process_group_has_members(pgid: u32) -> bool {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        // Keep enforcing the deadline if process-group membership cannot be
        // observed. Treating the group as empty here would return success
        // while descendants were still running.
        return true;
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if !name.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        if stat_is_live_group_member(&stat, pgid) {
            return true;
        }
    }
    false
}

#[cfg(all(unix, not(target_os = "linux")))]
fn process_group_has_members(pgid: u32) -> bool {
    let group = format!("-{pgid}");
    Command::new("kill")
        .args(["-0", "--", group.as_str()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(not(unix))]
fn process_group_has_members(_pgid: u32) -> bool {
    false
}

/// `/proc/<pid>/stat` field 5 is the process group. `comm` may contain spaces
/// and parentheses, so fields are counted after the last `)`.
#[cfg(any(test, target_os = "linux"))]
fn process_group_id(stat: &str) -> Option<u32> {
    let (_, after_comm) = stat.rsplit_once(')')?;
    let mut fields = after_comm.split_whitespace();
    let _state = fields.next()?;
    let _ppid = fields.next()?;
    fields.next()?.parse().ok()
}

#[cfg(any(test, target_os = "linux"))]
fn process_state(stat: &str) -> Option<&str> {
    let (_, after_comm) = stat.rsplit_once(')')?;
    after_comm.split_whitespace().next()
}

#[cfg(target_os = "linux")]
fn parent_pid(stat: &str) -> Option<u32> {
    let (_, after_comm) = stat.rsplit_once(')')?;
    let mut fields = after_comm.split_whitespace();
    let _state = fields.next()?;
    fields.next()?.parse().ok()
}

/// A zombie still occupies its process group until something reaps it. It is
/// not running the command.
#[cfg(any(test, target_os = "linux"))]
fn stat_is_live_group_member(stat: &str, pgid: u32) -> bool {
    process_group_id(stat) == Some(pgid) && process_state(stat) != Some("Z")
}

fn refused_network_isolation(status: &SandboxStatus) -> Option<io::Error> {
    if !status.enabled || !status.requested.network_isolation || status.network_active {
        return None;
    }
    let fallback = status
        .fallback_reason
        .as_deref()
        .unwrap_or("network isolation unavailable");
    let detail = fallback
        .split("; ")
        .find(|part| part.contains("network isolation"))
        .unwrap_or(fallback);
    Some(io::Error::other(format!(
        "{detail}; refusing to run the command without network isolation"
    )))
}

fn completed_output(
    output: &Output,
    dangerously_disable_sandbox: Option<bool>,
    sandbox_status: SandboxStatus,
) -> BashCommandOutput {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let no_output_expected = Some(stdout.trim().is_empty() && stderr.trim().is_empty());
    let return_code_interpretation = output.status.code().and_then(|code| {
        if code == 0 {
            None
        } else {
            Some(format!("exit_code:{code}"))
        }
    });

    BashCommandOutput {
        stdout,
        stderr,
        raw_output_path: None,
        interrupted: false,
        is_image: None,
        background_task_id: None,
        backgrounded_by_user: None,
        assistant_auto_backgrounded: None,
        dangerously_disable_sandbox,
        return_code_interpretation,
        no_output_expected,
        structured_content: None,
        persisted_output_path: None,
        persisted_output_size: None,
        sandbox_status: Some(sandbox_status),
    }
}

fn timed_out_output(
    stdout: &[u8],
    stderr: &[u8],
    timeout_ms: u64,
    dangerously_disable_sandbox: Option<bool>,
    sandbox_status: SandboxStatus,
) -> BashCommandOutput {
    let stdout = String::from_utf8_lossy(stdout).into_owned();
    let captured_stderr = String::from_utf8_lossy(stderr);
    let stderr = if captured_stderr.trim().is_empty() {
        format!("Command exceeded timeout of {timeout_ms} ms")
    } else {
        format!(
            "{}\nCommand exceeded timeout of {timeout_ms} ms",
            captured_stderr.trim_end()
        )
    };

    BashCommandOutput {
        stdout,
        stderr,
        raw_output_path: None,
        interrupted: true,
        is_image: None,
        background_task_id: None,
        backgrounded_by_user: None,
        assistant_auto_backgrounded: None,
        dangerously_disable_sandbox,
        return_code_interpretation: Some(String::from("timeout")),
        no_output_expected: Some(true),
        structured_content: None,
        persisted_output_path: None,
        persisted_output_size: None,
        sandbox_status: Some(sandbox_status),
    }
}

fn sandbox_status_for_input(input: &BashCommandInput, cwd: &std::path::Path) -> SandboxStatus {
    let config = ConfigLoader::default_for(cwd).load().map_or_else(
        |_| SandboxConfig::default(),
        |runtime_config| runtime_config.sandbox().clone(),
    );
    let request = config.resolve_request(
        input.dangerously_disable_sandbox.map(|disabled| !disabled),
        input.namespace_restrictions,
        input.isolate_network,
        input.filesystem_mode,
        input.allowed_mounts.clone(),
    );
    resolve_sandbox_status_for_request(&request, cwd)
}

fn prepare_command(
    command: &str,
    cwd: &std::path::Path,
    sandbox_status: &SandboxStatus,
    create_dirs: bool,
) -> Command {
    if create_dirs {
        prepare_sandbox_dirs(cwd);
    }

    if let Some(launcher) = build_linux_sandbox_command(command, cwd, sandbox_status) {
        let mut prepared = Command::new(launcher.program);
        prepared.args(launcher.args);
        prepared.current_dir(cwd);
        prepared.envs(launcher.env);
        return prepared;
    }

    let mut prepared = Command::new("sh");
    prepared.arg("-lc").arg(command).current_dir(cwd);
    if sandbox_status.filesystem_active {
        prepared.env("HOME", cwd.join(".sandbox-home"));
        prepared.env("TMPDIR", cwd.join(".sandbox-tmp"));
    }
    prepared
}

fn prepare_sandbox_dirs(cwd: &std::path::Path) {
    let _ = std::fs::create_dir_all(cwd.join(".sandbox-home"));
    let _ = std::fs::create_dir_all(cwd.join(".sandbox-tmp"));
}

#[cfg(test)]
mod tests {
    use super::{execute_bash, BashCommandInput};
    use crate::sandbox::FilesystemIsolationMode;

    #[test]
    fn executes_simple_command() {
        let output = execute_bash(BashCommandInput {
            command: String::from("printf 'hello'"),
            timeout: Some(1_000),
            description: None,
            run_in_background: Some(false),
            dangerously_disable_sandbox: Some(false),
            namespace_restrictions: Some(false),
            isolate_network: Some(false),
            filesystem_mode: Some(FilesystemIsolationMode::WorkspaceOnly),
            allowed_mounts: None,
        })
        .expect("bash command should execute");

        assert_eq!(output.stdout, "hello");
        assert!(!output.interrupted);
        assert!(output.sandbox_status.is_some());
    }

    #[test]
    fn default_sandbox_returns_command_stdout() {
        let _guard = crate::test_env_lock();
        let output = execute_bash(BashCommandInput {
            command: String::from("printf 'hello'"),
            timeout: None,
            description: None,
            run_in_background: None,
            dangerously_disable_sandbox: None,
            namespace_restrictions: None,
            isolate_network: None,
            filesystem_mode: None,
            allowed_mounts: None,
        })
        .expect("default sandbox should still run the command");

        assert_eq!(output.stdout, "hello");
        assert!(!output.interrupted);
        assert!(output.sandbox_status.is_some());
    }

    #[test]
    fn disables_sandbox_when_requested() {
        let output = execute_bash(BashCommandInput {
            command: String::from("printf 'hello'"),
            timeout: Some(1_000),
            description: None,
            run_in_background: Some(false),
            dangerously_disable_sandbox: Some(true),
            namespace_restrictions: None,
            isolate_network: None,
            filesystem_mode: None,
            allowed_mounts: None,
        })
        .expect("bash command should execute");

        assert!(!output.sandbox_status.expect("sandbox status").enabled);
    }

    fn unsandboxed(command: &str, timeout: Option<u64>, background: bool) -> BashCommandInput {
        BashCommandInput {
            command: command.to_string(),
            timeout,
            description: None,
            run_in_background: Some(background),
            dangerously_disable_sandbox: Some(true),
            namespace_restrictions: None,
            isolate_network: None,
            filesystem_mode: None,
            allowed_mounts: None,
        }
    }

    #[test]
    fn times_out_and_records_nonzero_exit_codes() {
        let started = std::time::Instant::now();
        let timed_out = execute_bash(unsandboxed("sleep 5", Some(200), false))
            .expect("timed out command should return");
        assert!(timed_out.interrupted);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "timeout returned only after {:?}, so the child kept the pipes open",
            started.elapsed()
        );
        assert_eq!(
            timed_out.return_code_interpretation.as_deref(),
            Some("timeout")
        );
        assert!(timed_out.stderr.contains("timeout"));

        let failed =
            execute_bash(unsandboxed("exit 7", Some(1_000), false)).expect("exit should return");
        assert!(!failed.interrupted);
        assert_eq!(
            failed.return_code_interpretation.as_deref(),
            Some("exit_code:7")
        );
        assert_eq!(failed.no_output_expected, Some(true));
    }

    #[test]
    fn returns_background_task_without_waiting_for_output() {
        let output = execute_bash(unsandboxed("sleep 30", None, true))
            .expect("background command should spawn");
        let pid = output
            .background_task_id
            .expect("background task id should be recorded");
        let _ = std::process::Command::new("kill").arg(&pid).status();

        assert_eq!(output.stdout, "");
        assert_eq!(output.no_output_expected, Some(true));
        assert!(!output.interrupted);
        assert!(output.sandbox_status.is_some());
    }

    #[test]
    fn parses_process_group_id_after_comm_with_spaces() {
        assert_eq!(
            super::process_group_id("1249 (sh) S 298 1249 1249 0"),
            Some(1249)
        );
        assert_eq!(super::process_group_id("12 (my proc) S 1 99 12"), Some(99));
        assert_eq!(super::process_group_id("not a stat line"), None);
    }

    #[test]
    fn zombie_process_group_members_are_not_alive() {
        assert!(super::stat_is_live_group_member(
            "1249 (sleep) S 298 1249 1249 0",
            1249
        ));
        assert!(!super::stat_is_live_group_member(
            "1249 (sleep) Z 298 1249 1249 0",
            1249
        ));
        assert!(!super::stat_is_live_group_member(
            "12 (my proc) Z 1 99 12",
            99
        ));
    }

    #[test]
    fn timed_command_drains_stdout_and_stderr_together() {
        let _guard = crate::test_env_lock();
        // Each iteration writes a full pipe buffer to stdout and then stderr.
        // A reader that ignores either stream lets that write block, the
        // interpreter never exits, and the deadline fires.
        let output = execute_bash(unsandboxed(
            r#"python3 -c 'import sys
chunk = 65536
data_out = b"o" * chunk
data_err = b"e" * chunk
for _ in range(6):
    sys.stdout.buffer.write(data_out)
    sys.stdout.buffer.flush()
    sys.stderr.buffer.write(data_err)
    sys.stderr.buffer.flush()
'"#,
            Some(5_000),
            false,
        ))
        .expect("large command should finish within the timeout");

        assert!(
            !output.interrupted,
            "full pipes should be drained, got stderr {}",
            output.stderr
        );
        assert_eq!(output.stdout.len(), 6 * 65_536);
        assert_eq!(output.stderr.len(), 6 * 65_536);
        assert!(output.stdout.bytes().all(|byte| byte == b'o'));
        assert!(output.stderr.bytes().all(|byte| byte == b'e'));
    }

    #[test]
    fn timeout_covers_descendants_after_the_shell_exits() {
        let _guard = crate::test_env_lock();
        let token = unique_token("group");
        let started = std::time::Instant::now();
        let output = execute_bash(unsandboxed(
            &format!("sh -c 'sleep 5' {token} >/dev/null 2>&1 & echo armed"),
            Some(300),
            false,
        ))
        .expect("descendant timeout should return");
        let elapsed = started.elapsed();
        let mut cleanup = KillMatching(Some(token.clone()));

        assert!(
            output.interrupted,
            "descendant was still running after the shell exited"
        );
        assert!(output.stdout.contains("armed"));
        assert_eq!(
            output.return_code_interpretation.as_deref(),
            Some("timeout")
        );
        assert!(
            elapsed >= std::time::Duration::from_millis(200),
            "timeout returned after {elapsed:?}, before the deadline"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "timeout returned after {elapsed:?}; the shell exit ended the wait"
        );
        assert_no_process_matching(&token);
        cleanup.disarm();
    }

    #[test]
    fn timeout_kills_a_detached_session_leader() {
        let _guard = crate::test_env_lock();
        let token = unique_token("setsid");
        let started = std::time::Instant::now();
        let output = execute_bash(unsandboxed(
            &format!("setsid sh -c 'sleep 30' {token} >/dev/null 2>&1 </dev/null & echo armed"),
            Some(300),
            false,
        ))
        .expect("detached descendant should be bounded by the timeout");
        let elapsed = started.elapsed();
        let mut cleanup = KillMatching(Some(token.clone()));

        assert!(
            output.interrupted,
            "setsid child escaped the timeout, stdout {} stderr {}",
            output.stdout, output.stderr
        );
        assert!(output.stdout.contains("armed"));
        assert_eq!(
            output.return_code_interpretation.as_deref(),
            Some("timeout")
        );
        assert!(
            elapsed >= std::time::Duration::from_millis(200),
            "timeout returned after {elapsed:?}, before the deadline"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "timeout returned after {elapsed:?}"
        );
        assert_no_process_matching(&token);
        cleanup.disarm();
    }

    #[test]
    fn finished_command_is_not_a_timeout_when_it_leaves_a_zombie() {
        let _guard = crate::test_env_lock();
        let started = std::time::Instant::now();
        let output = execute_bash(unsandboxed(
            r#"python3 -c 'import os,sys
pid = os.fork()
if pid == 0:
    os._exit(0)
sys.stdout.write("done\n")
sys.stdout.flush()
os._exit(0)
'"#,
            Some(2_000),
            false,
        ))
        .expect("zombie child should not fail the command");
        let elapsed = started.elapsed();

        assert!(
            !output.interrupted,
            "zombie was treated as a running descendant: {}",
            output.stderr
        );
        assert!(output.stdout.contains("done"));
        assert_eq!(output.return_code_interpretation, None);
        assert!(
            elapsed < std::time::Duration::from_secs(1),
            "returned after {elapsed:?}; an unreaped zombie kept the deadline open"
        );
    }

    #[test]
    fn timeout_still_bounds_a_descendant_holding_the_output_pipes() {
        let _guard = crate::test_env_lock();
        let token = unique_token("pipe");
        let started = std::time::Instant::now();
        let output = execute_bash(unsandboxed(
            &format!("sh -c 'sleep 5' {token} & echo armed"),
            Some(300),
            false,
        ))
        .expect("piped descendant should not block past the timeout");
        let elapsed = started.elapsed();
        let mut cleanup = KillMatching(Some(token.clone()));

        assert!(output.interrupted, "{}", output.stderr);
        assert!(output.stdout.contains("armed"));
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "waited {elapsed:?} for a descendant that kept stdout open"
        );
        assert_no_process_matching(&token);
        cleanup.disarm();
    }

    #[test]
    fn descendant_that_exits_before_the_deadline_is_not_a_timeout() {
        let _guard = crate::test_env_lock();
        let started = std::time::Instant::now();
        let output = execute_bash(unsandboxed("sleep 0.4 & echo ready", Some(2_000), false))
            .expect("short descendant should finish");
        let elapsed = started.elapsed();

        assert!(!output.interrupted, "{}", output.stderr);
        assert!(output.stdout.contains("ready"));
        assert_eq!(output.return_code_interpretation, None);
        assert!(
            elapsed >= std::time::Duration::from_millis(250),
            "returned after {elapsed:?}, without waiting for the descendant"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "returned after {elapsed:?}"
        );
    }

    #[test]
    fn failed_sandbox_probe_refuses_requested_network_isolation() {
        let _guard = crate::test_env_lock();
        let _probe = crate::sandbox::override_namespace_support(false, true);
        let marker = std::env::temp_dir().join(format!(
            "claw-net-refuse-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        let _ = std::fs::remove_file(&marker);
        let error = execute_bash(BashCommandInput {
            command: format!("touch {}", marker.display()),
            timeout: Some(1_000),
            description: None,
            run_in_background: Some(false),
            dangerously_disable_sandbox: Some(false),
            namespace_restrictions: Some(true),
            isolate_network: Some(true),
            filesystem_mode: Some(FilesystemIsolationMode::Off),
            allowed_mounts: None,
        })
        .expect_err("requested network isolation must fail closed");
        let ran = marker.exists();
        let _ = std::fs::remove_file(&marker);

        assert_eq!(
            error.to_string(),
            "network isolation unavailable (`unshare` cannot create a user, mount, and pid namespace); refusing to run the command without network isolation"
        );
        assert!(!ran, "command ran without network isolation");
    }

    #[test]
    fn failed_sandbox_probe_still_runs_when_network_isolation_is_off() {
        let _guard = crate::test_env_lock();
        let _probe = crate::sandbox::override_namespace_support(false, true);
        let output = execute_bash(BashCommandInput {
            command: String::from("printf 'hello'"),
            timeout: Some(1_000),
            description: None,
            run_in_background: Some(false),
            dangerously_disable_sandbox: Some(false),
            namespace_restrictions: Some(true),
            isolate_network: Some(false),
            filesystem_mode: Some(FilesystemIsolationMode::Off),
            allowed_mounts: None,
        })
        .expect("namespace fallback should still run the command");

        assert_eq!(output.stdout, "hello");
        let status = output.sandbox_status.expect("sandbox status");
        assert!(!status.network_active);
        assert!(!status.active);
    }

    fn unique_token(label: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        format!("claw-{label}-{}-{nanos}", std::process::id())
    }

    fn assert_no_process_matching(token: &str) {
        #[cfg(target_os = "linux")]
        {
            for _ in 0..50 {
                let output = std::process::Command::new("ps")
                    .args(["-eo", "args"])
                    .stdin(std::process::Stdio::null())
                    .output()
                    .expect("ps should list processes");
                let text = String::from_utf8_lossy(&output.stdout);
                if !text.lines().any(|line| line.contains(token)) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            panic!("process matching {token} was still running after the timeout");
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = token;
        }
    }

    struct KillMatching(Option<String>);

    impl KillMatching {
        fn disarm(&mut self) {
            self.0 = None;
        }
    }

    impl Drop for KillMatching {
        fn drop(&mut self) {
            if let Some(token) = self.0.take() {
                let _ = std::process::Command::new("pkill")
                    .args(["-KILL", "-f", &token])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
            }
        }
    }
}
