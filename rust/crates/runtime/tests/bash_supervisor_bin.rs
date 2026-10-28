//! Forces `cargo test` to build the real `bash-supervisor` binary.
//!
//! `cargo test` otherwise emits only the bin's test harness under `deps/`.
//! Timed commands need the non-test executable, which Cargo places beside
//! `target/debug` when an integration test names `CARGO_BIN_EXE_<name>`.

#[test]
fn cargo_builds_the_bash_supervisor_binary() {
    let program = env!("CARGO_BIN_EXE_bash-supervisor");
    assert!(
        std::path::Path::new(program).is_file(),
        "bash-supervisor was not built at {program}"
    );
    let output = std::process::Command::new(program)
        .arg("1000")
        .arg("exit 0")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .expect("bash-supervisor should run");
    assert!(
        output.status.success(),
        "bash-supervisor probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
