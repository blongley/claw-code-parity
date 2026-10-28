#[cfg(all(test, unix))]
use std::cell::Cell;
use std::env;
use std::io;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::time::Duration;

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
            &prepared,
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
    command: &Command,
    timeout_ms: u64,
    dangerously_disable_sandbox: Option<bool>,
    sandbox_status: SandboxStatus,
) -> io::Result<BashCommandOutput> {
    // Process-group tracking cannot see `setsid` children. Tests force that
    // mode to prove the command is refused before it starts. Production
    // creates the pid namespace itself and fail-closes when it cannot.
    #[cfg(all(test, unix))]
    if supervision_override_is_process_group() {
        return Err(containment_unavailable());
    }
    #[cfg(target_os = "linux")]
    {
        execute_contained(
            command,
            timeout_ms,
            dangerously_disable_sandbox,
            sandbox_status,
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (
            command,
            timeout_ms,
            dangerously_disable_sandbox,
            sandbox_status,
        );
        Err(containment_unavailable())
    }
}

#[cfg(target_os = "linux")]
fn execute_contained(
    command: &Command,
    timeout_ms: u64,
    dangerously_disable_sandbox: Option<bool>,
    sandbox_status: SandboxStatus,
) -> io::Result<BashCommandOutput> {
    let (outcome, captured) =
        crate::contain::run_timed(command, Duration::from_millis(timeout_ms))?;
    match outcome {
        crate::contain::Containment::Finished(code) => Ok(completed_output(
            &Output {
                status: ExitStatus::from_raw(code << 8),
                stdout: captured.stdout,
                stderr: captured.stderr,
            },
            dangerously_disable_sandbox,
            sandbox_status,
        )),
        crate::contain::Containment::TimedOut => Ok(timed_out_output(
            &captured.stdout,
            &captured.stderr,
            timeout_ms,
            dangerously_disable_sandbox,
            sandbox_status,
        )),
        crate::contain::Containment::Unkillable { detail } => Ok(unkillable_output(
            &captured.stdout,
            &captured.stderr,
            &detail,
            dangerously_disable_sandbox,
            sandbox_status,
        )),
        crate::contain::Containment::Unavailable { detail } => Err(unavailable_error(&detail)),
        crate::contain::Containment::Failed(message) => Err(io::Error::other(message)),
    }
}

#[cfg(any(test, not(target_os = "linux")))]
fn containment_unavailable() -> io::Error {
    unavailable_error("")
}

fn unavailable_error(detail: &str) -> io::Error {
    let detail = if detail.is_empty() {
        "no pid namespace"
    } else {
        detail
    };
    io::Error::other(format!(
        "cannot contain detached child processes ({detail}); refusing to start the command"
    ))
}

#[cfg(all(test, unix))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Supervision {
    ProcessGroup,
}

#[cfg(all(test, unix))]
thread_local! {
    static SUPERVISION_OVERRIDE: Cell<Option<Supervision>> = const { Cell::new(None) };
}

#[cfg(all(test, unix))]
#[must_use = "the supervision override ends when this guard is dropped"]
struct SupervisionOverride;

#[cfg(all(test, unix))]
fn override_supervision(mode: Supervision) -> SupervisionOverride {
    SUPERVISION_OVERRIDE.with(|cell| cell.set(Some(mode)));
    SupervisionOverride
}

#[cfg(all(test, unix))]
impl Drop for SupervisionOverride {
    fn drop(&mut self) {
        SUPERVISION_OVERRIDE.with(|cell| cell.set(None));
    }
}

#[cfg(all(test, unix))]
fn supervision_override_is_process_group() -> bool {
    SUPERVISION_OVERRIDE.with(Cell::get) == Some(Supervision::ProcessGroup)
}

/// `/proc/<pid>/stat` field 5 is the process group. `comm` may contain spaces
/// and parentheses, so fields are counted after the last `)`.
#[cfg(test)]
fn process_group_id(stat: &str) -> Option<u32> {
    let (_, after_comm) = stat.rsplit_once(')')?;
    let mut fields = after_comm.split_whitespace();
    let _state = fields.next()?;
    let _ppid = fields.next()?;
    fields.next()?.parse().ok()
}

#[cfg(test)]
fn process_state(stat: &str) -> Option<&str> {
    let (_, after_comm) = stat.rsplit_once(')')?;
    after_comm.split_whitespace().next()
}

/// A zombie still occupies its process group until something reaps it. It is
/// not running the command.
#[cfg(test)]
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

#[cfg(target_os = "linux")]
fn unkillable_output(
    stdout: &[u8],
    stderr: &[u8],
    detail: &str,
    dangerously_disable_sandbox: Option<bool>,
    sandbox_status: SandboxStatus,
) -> BashCommandOutput {
    let stdout = String::from_utf8_lossy(stdout).into_owned();
    let captured_stderr = String::from_utf8_lossy(stderr);
    let status = crate::contain::UNKILLABLE_STATUS;
    let mut stderr = if captured_stderr.trim().is_empty() {
        String::new()
    } else {
        captured_stderr.trim_end().to_string()
    };
    if !detail.is_empty() && !stderr.contains(detail) {
        if !stderr.is_empty() {
            stderr.push('\n');
        }
        stderr.push_str(detail);
    }
    if !stderr.contains(status) {
        if !stderr.is_empty() {
            stderr.push('\n');
        }
        stderr.push_str(status);
    }
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
        return_code_interpretation: Some(status.to_string()),
        no_output_expected: Some(true),
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
        #[cfg(unix)]
        if let Ok(raw) = pid.parse::<i32>() {
            if let Some(pid) = rustix::process::Pid::from_raw(raw) {
                let _ = rustix::process::kill_process(pid, rustix::process::Signal::KILL);
            }
        }

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

    #[cfg(target_os = "linux")]
    #[test]
    fn timed_command_creates_no_status_file() {
        let _guard = crate::test_env_lock();
        let before = claw_status_files();
        let output = execute_bash(unsandboxed(
            "if ls /tmp/claw-bash-*status* >/dev/null 2>&1; then echo status-file; else echo no-status-file; fi; if ls -l /proc/self/fd /proc/1/fd 2>/dev/null | grep -F -e claw-bash -e .status >/dev/null; then echo status-fd; else echo no-status-fd; fi",
            Some(2_000),
            false,
        ))
        .expect("status probe should run");
        let after = claw_status_files();

        assert_eq!(before, after, "timed bash created a status file");
        assert!(
            output.stdout.contains("no-status-file"),
            "a status file was visible to the command: {}",
            output.stdout
        );
        assert!(
            output.stdout.contains("no-status-fd"),
            "a status file was open in the command: {}",
            output.stdout
        );
        assert!(!output.interrupted, "{}", output.stderr);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn timed_command_inherits_no_fds_above_stdio() {
        let _guard = crate::test_env_lock();
        let output = execute_bash(unsandboxed("exec ls -1 /proc/self/fd", Some(2_000), false))
            .expect("fd probe should run");
        assert!(!output.interrupted, "{}", output.stderr);
        let fds: Vec<i32> = output
            .stdout
            .lines()
            .map(|line| line.trim().parse().expect("fd number"))
            .collect();
        assert!(
            fds.iter().all(|fd| *fd <= 3),
            "command inherited a descriptor above stdio: {fds:?}\n{}",
            output.stderr
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn release_waits_for_the_parent_death_ack() {
        let _guard = crate::test_env_lock();
        let _inject = crate::contain::inject_arming_failure();
        let marker = std::env::temp_dir().join(unique_token("arm-fail"));
        let _ = std::fs::remove_file(&marker);
        let error = execute_bash(unsandboxed(
            &format!("touch {}", marker.display()),
            Some(2_000),
            false,
        ))
        .expect_err("arming failure must refuse the command");

        assert!(!marker.exists(), "the command ran without an armed init");
        assert_eq!(
            crate::contain::release_count(),
            0,
            "the release byte was written before the ack"
        );
        assert!(
            error
                .to_string()
                .contains("did not arm parent-death signal"),
            "{error}"
        );
        assert!(error.to_string().contains("refusing to start"), "{error}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn namespace_failure_refuses_before_the_command_starts() {
        let _guard = crate::test_env_lock();
        let _inject = crate::contain::inject_namespace_failure();
        let marker = std::env::temp_dir().join(unique_token("ns-fail"));
        let _ = std::fs::remove_file(&marker);
        let error = execute_bash(unsandboxed(
            &format!("touch {}", marker.display()),
            Some(2_000),
            false,
        ))
        .expect_err("a missing pid namespace must refuse the command");

        assert!(!marker.exists(), "the command ran without a pid namespace");
        assert_eq!(crate::contain::release_count(), 0);
        assert_eq!(
            error.to_string(),
            super::unavailable_error("clone3: Operation not permitted").to_string()
        );
    }

    #[cfg(target_os = "linux")]
    fn claw_status_files() -> Vec<String> {
        let mut names = Vec::new();
        let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
            return names;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.contains("claw-bash") || name.contains("claw-supervisor") {
                names.push(name);
            }
        }
        names.sort();
        names
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn user_exit_124_is_not_a_timeout() {
        let output = execute_bash(unsandboxed("exit 124", Some(1_000), false))
            .expect("exit 124 should return");
        assert!(!output.interrupted);
        assert_eq!(
            output.return_code_interpretation.as_deref(),
            Some("exit_code:124")
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn unkillable_status_is_not_exit_127() {
        let output =
            super::unkillable_output(&[], &[], "", None, crate::sandbox::SandboxStatus::default());
        assert!(output.interrupted);
        assert_eq!(
            output.return_code_interpretation.as_deref(),
            Some("processes could not be killed")
        );
        assert!(output.stderr.contains("processes could not be killed"));
        assert_ne!(
            output.return_code_interpretation.as_deref(),
            Some("exit_code:127")
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn kill_stop_of_the_command_group_does_not_freeze_the_supervisor() {
        let _guard = crate::test_env_lock();
        let started = std::time::Instant::now();
        let output = execute_bash(unsandboxed(
            "kill -STOP 0; echo continued",
            Some(400),
            false,
        ))
        .expect("stopped command should still hit the timeout");
        let elapsed = started.elapsed();

        assert!(
            output.interrupted,
            "supervisor did not time out: {}",
            output.stderr
        );
        assert!(
            !output.stdout.contains("continued"),
            "the shell kept running after kill -STOP 0"
        );
        assert_eq!(
            output.return_code_interpretation.as_deref(),
            Some("timeout")
        );
        assert!(
            elapsed >= std::time::Duration::from_millis(300),
            "timeout returned after {elapsed:?}, before the deadline"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "timeout returned after {elapsed:?}; kill -STOP 0 reached the supervisor"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn setsid_child_that_moves_to_the_parent_cgroup_is_killed() {
        let _guard = crate::test_env_lock();
        let token = unique_token("cgroup-escape");
        let marker = std::env::temp_dir().join(format!("{token}-cgroup"));
        let _ = std::fs::remove_file(&marker);
        let mut cleanup = KillMatching(Some(token.clone()));
        let started = std::time::Instant::now();
        let marker_path = marker.display().to_string();
        // setsid plus a cgroup migration. `cgroup.procs` takes the pid in the
        // writer's pid namespace, which is `getpid` inside the command.
        // A parent domain with subtree controllers returns EBUSY, so the child
        // also creates a cgroup of its own and moves there. Either way it is
        // still in the pid namespace.
        let command = format!(
            r#"python3 -c 'import os, pathlib
token = "{token}"
marker = pathlib.Path("{marker_path}")
pid = os.fork()
if pid != 0:
    os._exit(0)
os.setsid()
def current():
    for line in pathlib.Path("/proc/self/cgroup").read_text().splitlines():
        if line.startswith("0::"):
            return line[3:].strip()
    return ""
def migrate(path):
    with open(path, "w") as handle:
        handle.write(str(os.getpid()))
original = current()
errors = []
landed = original
parent = os.path.dirname(original)
try:
    migrate("/sys/fs/cgroup" + parent + "/cgroup.procs")
    landed = current()
except OSError as err:
    errors.append("parent:" + str(err))
if landed == original:
    nest = "/sys/fs/cgroup" + original + "/claw-{token}"
    try:
        os.mkdir(nest)
        migrate(nest + "/cgroup.procs")
        landed = current()
    except OSError as err:
        errors.append("nest:" + str(err))
if landed != original:
    marker.write_text("moved " + landed)
else:
    marker.write_text("move-failed:" + ",".join(errors))
os.execv("/bin/sleep", [token, "30"])
'"#
        );
        let output = execute_bash(unsandboxed(&command, Some(400), false))
            .expect("setsid child should be bounded by the pid namespace");
        let elapsed = started.elapsed();
        let recorded = std::fs::read_to_string(&marker).unwrap_or_default();
        let _ = std::fs::remove_file(&marker);
        if let Some(landed) = recorded.strip_prefix("moved ") {
            let nest = std::path::Path::new("/sys/fs/cgroup").join(landed.trim_start_matches('/'));
            if nest
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(&token))
            {
                let _ = std::fs::remove_dir(&nest);
            }
        }

        assert!(
            output.interrupted,
            "setsid child survived: stdout {} stderr {} marker {recorded}",
            output.stdout, output.stderr
        );
        assert_eq!(
            output.return_code_interpretation.as_deref(),
            Some("timeout")
        );
        assert!(
            recorded.starts_with("moved ")
                || (recorded.starts_with("move-failed:")
                    && (recorded.contains("Permission denied")
                        || recorded.contains("Operation not permitted")
                        || recorded.contains("Read-only file system"))),
            "setsid child did not migrate, and the cgroup was writable: {recorded}"
        );
        assert!(
            elapsed >= std::time::Duration::from_millis(300),
            "timeout returned after {elapsed:?}, before the deadline"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "timeout returned after {elapsed:?}; marker {recorded}"
        );
        assert_no_process_matching(&token);
        cleanup.disarm();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn process_group_fallback_refuses_to_report_success() {
        let _guard = crate::test_env_lock();
        let _mode = super::override_supervision(super::Supervision::ProcessGroup);
        let marker = std::env::temp_dir().join(unique_token("pg-ran"));
        let _ = std::fs::remove_file(&marker);
        let started = std::time::Instant::now();
        let error = execute_bash(unsandboxed(
            &format!("touch {}", marker.display()),
            Some(1_000),
            false,
        ))
        .expect_err("process group cannot contain detached children");

        assert!(
            started.elapsed() < std::time::Duration::from_millis(500),
            "returned after {:?}",
            started.elapsed()
        );
        assert_eq!(
            error.to_string(),
            super::containment_unavailable().to_string()
        );
        assert!(
            !marker.exists(),
            "process-group mode started the command before refusing"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn process_group_fallback_refuses_a_detached_setsid_child() {
        let _guard = crate::test_env_lock();
        let _mode = super::override_supervision(super::Supervision::ProcessGroup);
        let token = unique_token("pg-setsid");
        let marker = std::env::temp_dir().join(format!("{token}-ran"));
        let _ = std::fs::remove_file(&marker);
        let started = std::time::Instant::now();
        let error = execute_bash(unsandboxed(
            &format!(
                "setsid sh -c 'sleep 30' {token} >/dev/null 2>&1 </dev/null & touch {} ; echo armed",
                marker.display()
            ),
            Some(300),
            false,
        ))
        .expect_err("a detached child must not be started without containment");
        let elapsed = started.elapsed();
        let _cleanup = KillMatching(Some(token.clone()));

        assert_eq!(
            error.to_string(),
            super::containment_unavailable().to_string()
        );
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "returned after {elapsed:?}; the command was started"
        );
        assert!(!marker.exists(), "setsid command was started");
        assert_no_process_matching(&token);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn timeout_kills_a_child_forked_during_teardown() {
        let _guard = crate::test_env_lock();
        let token = unique_token("fork-teardown");
        let started = std::time::Instant::now();
        let output = execute_bash(unsandboxed(
            &format!(
                r#"python3 -c 'import os, time
token = "{token}"
while True:
    pid = os.fork()
    if pid == 0:
        os.setsid()
        os.execv("/bin/sleep", [token, "30"])
    time.sleep(0.01)
'"#
            ),
            Some(400),
            false,
        ))
        .expect("forking descendant should be bounded by the timeout");
        let elapsed = started.elapsed();
        let mut cleanup = KillMatching(Some(token.clone()));

        assert!(
            output.interrupted,
            "forked children survived teardown: {}",
            output.stderr
        );
        assert_eq!(
            output.return_code_interpretation.as_deref(),
            Some("timeout")
        );
        assert!(
            elapsed >= std::time::Duration::from_millis(300),
            "timeout returned after {elapsed:?}, before the deadline"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "timeout returned after {elapsed:?}"
        );
        assert_no_process_matching(&token);
        cleanup.disarm();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cleanup_never_runs_kill_from_path() {
        const CHILD: &str = "CLAWD_SUPERVISOR_KILL_CHILD";
        const MARKER: &str = "CLAWD_SUPERVISOR_KILL_MARKER";
        if std::env::var_os(CHILD).is_none() {
            let dir = std::env::temp_dir().join(unique_token("path-kill"));
            std::fs::create_dir_all(&dir).expect("kill shadow dir");
            let marker = dir.join("kill-ran");
            let done = dir.join("done");
            write_executable(
                &dir.join("kill"),
                "#!/bin/sh\ntouch \"$CLAWD_SUPERVISOR_KILL_MARKER\"\nexit 0\n",
            );
            let mut path = std::ffi::OsString::from(dir.as_os_str());
            path.push(":");
            if let Some(current) = std::env::var_os("PATH") {
                path.push(current);
            }
            let output = std::process::Command::new(
                std::env::current_exe().expect("test binary should be available"),
            )
            .args([
                "--exact",
                "bash::tests::cleanup_never_runs_kill_from_path",
                "--test-threads=1",
            ])
            .env(CHILD, "1")
            .env(MARKER, &marker)
            .env("PATH", &path)
            .output()
            .expect("kill shadow test should re-exec");
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let ran = marker.exists();
            let finished = done.exists();
            let _ = std::fs::remove_dir_all(&dir);
            assert!(
                output.status.success() && finished && !ran,
                "status {:?} finished {finished} kill-ran {ran}\n{stdout}\n{stderr}",
                output.status.code()
            );
            return;
        }

        let _guard = crate::test_env_lock();
        let marker = std::env::var(MARKER).expect("marker path");
        let token = unique_token("path-kill-sleep");
        let mut cleanup = KillMatching(Some(token.clone()));
        let output = execute_bash(unsandboxed(
            &format!(r#"python3 -c 'import os; os.execv("/bin/sleep", ["{token}", "30"])'"#),
            Some(200),
            false,
        ))
        .expect("timed command should return without running kill");
        assert!(output.interrupted, "{}", output.stderr);
        assert_eq!(
            output.return_code_interpretation.as_deref(),
            Some("timeout")
        );
        assert!(
            !std::path::Path::new(&marker).exists(),
            "cleanup executed kill from PATH"
        );
        assert_no_process_matching(&token);
        let done = std::path::Path::new(&marker).with_file_name("done");
        std::fs::write(done, "ok").expect("done marker");
        cleanup.disarm();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn supervisor_does_not_execute_a_path_shadowed_python() {
        const CHILD: &str = "CLAWD_SUPERVISOR_SHADOW_CHILD";
        const MARKER: &str = "CLAWD_SUPERVISOR_SHADOW_MARKER";
        if std::env::var_os(CHILD).is_none() {
            let dir = std::env::temp_dir().join(unique_token("py-shadow"));
            std::fs::create_dir_all(&dir).expect("shadow dir should be created");
            let marker = dir.join("python-ran");
            let done = dir.join("done");
            write_executable(
                &dir.join("python3"),
                "#!/bin/sh\ntouch \"$CLAWD_SUPERVISOR_SHADOW_MARKER\"\nexit 1\n",
            );
            let site = "import os\np=os.environ.get('CLAWD_SUPERVISOR_SHADOW_MARKER')\nif p:\n    open(p,'a').close()\n";
            std::fs::write(dir.join("sitecustomize.py"), site).expect("sitecustomize");
            std::fs::write(dir.join("ctypes.py"), site).expect("ctypes shadow");
            let mut path = std::ffi::OsString::from(dir.as_os_str());
            path.push(":");
            if let Some(current) = std::env::var_os("PATH") {
                path.push(current);
            }
            let output = std::process::Command::new(
                std::env::current_exe().expect("test binary should be available"),
            )
            .args([
                "--exact",
                "bash::tests::supervisor_does_not_execute_a_path_shadowed_python",
                "--test-threads=1",
            ])
            .current_dir(&dir)
            .env(CHILD, "1")
            .env(MARKER, &marker)
            .env("PATH", &path)
            .output()
            .expect("shadow test should re-exec");
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let ran = marker.exists();
            let finished = done.exists();
            let _ = std::fs::remove_dir_all(&dir);
            assert!(
                output.status.success() && finished && !ran,
                "status {:?} finished {finished} python-ran {ran}\n{stdout}\n{stderr}",
                output.status.code()
            );
            return;
        }

        let _guard = crate::test_env_lock();
        let marker = std::env::var(MARKER).expect("marker path");
        let output = execute_bash(unsandboxed("printf hello", Some(1_000), false))
            .expect("command should run without python");
        assert_eq!(output.stdout, "hello");
        assert!(
            !std::path::Path::new(&marker).exists(),
            "supervisor executed python3"
        );
        let done = std::path::Path::new(&marker).with_file_name("done");
        std::fs::write(done, "ok").expect("done marker");
    }

    #[cfg(target_os = "linux")]
    fn write_executable(path: &std::path::Path, contents: &str) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(path, contents).expect("executable script should be written");
        let mut permissions = std::fs::metadata(path)
            .expect("executable metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("executable bit");
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
