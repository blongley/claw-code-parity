//! Forces `cargo test` to build the real `bash-supervisor` binary.
//!
//! `cargo test` otherwise emits only the bin's test harness under `deps/`.
//! Timed commands need the non-test executable, which Cargo places beside
//! `target/debug` when an integration test names `CARGO_BIN_EXE_<name>`.

#[test]
fn cargo_builds_the_bash_supervisor_binary() {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let program = env!("CARGO_BIN_EXE_bash-supervisor");
    assert!(
        std::path::Path::new(program).is_file(),
        "bash-supervisor was not built at {program}"
    );
    let status_path = std::env::temp_dir().join(format!(
        "claw-supervisor-probe-{}.status",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&status_path);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    options.mode(0o600);
    options
        .open(&status_path)
        .expect("status file")
        .write_all(b"")
        .expect("status file should be created");
    let output = std::process::Command::new(program)
        .arg(&status_path)
        .arg("1000")
        .arg("exit 0")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("bash-supervisor should run");
    let text = std::fs::read_to_string(&status_path).unwrap_or_default();
    let _ = std::fs::remove_file(&status_path);
    assert!(
        output.status.success() && text.lines().any(|line| line == "result ok 0"),
        "bash-supervisor probe failed: status {text:?} stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
