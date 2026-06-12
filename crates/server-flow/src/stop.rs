use std::io::Write;
use std::process::Child;
use std::time::{Duration, Instant};

use crate::error::StopError;
use crate::process_tree::{force_kill_process_tree_by_pid, is_process_alive};
use crate::spec::{GracefulStopSpec, ProcessStopStrategy};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopOutcome {
    pub pid: u32,
    pub graceful_attempted: bool,
    pub forced: bool,
    pub exit_code: Option<i32>,
}

pub fn stop_local_process(
    child: &mut Child,
    graceful: Option<&GracefulStopSpec>,
    strategy: ProcessStopStrategy,
) -> Result<StopOutcome, StopError> {
    let pid = child.id();
    let mut graceful_attempted = false;

    match strategy {
        ProcessStopStrategy::GracefulThenForce => {
            let graceful = graceful.ok_or(StopError::GracefulStopUnsupported)?;
            graceful_attempted = true;
            send_stdin_line(child, &graceful.stdin_line)?;
            if wait_for_exit(child, graceful.timeout, graceful.poll_interval)? {
                let status = child.try_wait().map_err(|e| StopError::Io(e.to_string()))?;
                return Ok(StopOutcome {
                    pid,
                    graceful_attempted,
                    forced: false,
                    exit_code: status.and_then(|status| status.code()),
                });
            }
            force_kill_process_tree_by_pid(pid).map_err(StopError::ForceKillFailed)?;
            let _ = child.wait();
            Ok(StopOutcome { pid, graceful_attempted, forced: true, exit_code: None })
        }
        ProcessStopStrategy::ForceOnly => {
            force_kill_process_tree_by_pid(pid).map_err(StopError::ForceKillFailed)?;
            let _ = child.wait();
            Ok(StopOutcome { pid, graceful_attempted, forced: true, exit_code: None })
        }
    }
}

pub fn stop_process_by_pid(
    pid: u32,
    graceful: Option<&GracefulStopSpec>,
    strategy: ProcessStopStrategy,
) -> Result<StopOutcome, StopError> {
    let graceful_attempted = graceful.is_some() && matches!(strategy, ProcessStopStrategy::GracefulThenForce);

    if graceful_attempted {
        return Err(StopError::GracefulStopUnsupported);
    }

    if !is_process_alive(pid) {
        return Ok(StopOutcome { pid, graceful_attempted: false, forced: false, exit_code: None });
    }

    force_kill_process_tree_by_pid(pid).map_err(StopError::ForceKillFailed)?;
    Ok(StopOutcome { pid, graceful_attempted: false, forced: true, exit_code: None })
}

fn send_stdin_line(child: &mut Child, line: &str) -> Result<(), StopError> {
    let stdin = child.stdin.as_mut().ok_or(StopError::GracefulStopUnsupported)?;
    stdin
        .write_all(format!("{}\n", line).as_bytes())
        .map_err(|e| StopError::Io(e.to_string()))?;
    stdin.flush().map_err(|e| StopError::Io(e.to_string()))
}

fn wait_for_exit(
    child: &mut Child,
    timeout: Duration,
    poll_interval: Duration,
) -> Result<bool, StopError> {
    let started = Instant::now();

    loop {
        if child.try_wait().map_err(|e| StopError::Io(e.to_string()))?.is_some() {
            return Ok(true);
        }
        if started.elapsed() >= timeout {
            return Ok(false);
        }
        std::thread::sleep(poll_interval);
    }
}

#[cfg(test)]
mod tests {
    use super::{stop_local_process, stop_process_by_pid};
    use crate::spec::{GracefulStopSpec, ProcessStopStrategy};
    use std::process::Command;
    use std::time::Duration;

    fn graceful_child() -> Command {
        #[cfg(windows)]
        {
            let mut command = Command::new("powershell");
            command.args([
                "-NoProfile",
                "-Command",
                "$line=[Console]::In.ReadLine(); if ($line -eq 'stop') { exit 0 } else { exit 7 }",
            ]);
            command
        }

        #[cfg(not(windows))]
        {
            let mut command = Command::new("sh");
            command.args(["-c", "read line; if [ \"$line\" = \"stop\" ]; then exit 0; else exit 7; fi"]);
            command
        }
    }

    #[test]
    fn graceful_stop_writes_stop_line_and_waits_for_exit() {
        let mut child = graceful_child()
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("child should spawn");

        let outcome = stop_local_process(
            &mut child,
            Some(&GracefulStopSpec::stdin_line("stop").with_timeout(Duration::from_secs(2))),
            ProcessStopStrategy::GracefulThenForce,
        )
        .expect("graceful stop should succeed");

        assert!(outcome.graceful_attempted);
        assert!(!outcome.forced);
        assert_eq!(outcome.exit_code, Some(0));
    }

    #[test]
    fn stop_process_by_pid_rejects_graceful_pid_only_stop() {
        let error = stop_process_by_pid(
            std::process::id(),
            Some(&GracefulStopSpec::stdin_line("stop")),
            ProcessStopStrategy::GracefulThenForce,
        )
        .expect_err("pid-only graceful stop should be unsupported");

        assert_eq!(error, crate::error::StopError::GracefulStopUnsupported);
    }
}
