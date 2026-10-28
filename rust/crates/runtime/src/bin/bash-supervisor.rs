//! Session leader that contains one timed shell command.
//!
//! Timed commands need a process that outlives `setsid`. This program forks a
//! pid-namespace init, records that init's pidfd before the command runs, and
//! exits only after init has been killed and waited on, or the failure is
//! recorded in the status file.

fn main() {
    std::process::exit(runtime::bash_supervisor_main());
}
