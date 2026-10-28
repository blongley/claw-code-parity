//! Session leader that contains one timed shell command.
//!
//! Timed commands need a process that outlives `setsid`. This program creates
//! a cgroup v2 before exec, or a pid namespace, and exits only after that
//! container is empty or the failure is recorded in the status file.

fn main() {
    std::process::exit(runtime::bash_supervisor_main());
}
