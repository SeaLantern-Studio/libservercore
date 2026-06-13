use std::net::{SocketAddr, TcpStream};
use std::process::Child;
use std::time::Duration;

use crate::error::StatusError;
use crate::process_tree::is_process_alive;
use crate::spec::PortProbeSpec;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalFlowState {
    Starting,
    Running,
    Stopped,
    Exited,
    Unknown,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortProbeStatus {
    pub port: u16,
    pub listening: bool,
}

#[derive(Debug)]
pub struct OwnedProcessHandle {
    child: Child,
}

impl OwnedProcessHandle {
    pub fn new(child: Child) -> Self {
        Self { child }
    }

    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn child(&self) -> &Child {
        &self.child
    }

    pub fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFlowStatus {
    pub state: LocalFlowState,
    pub pid: Option<u32>,
    pub running: bool,
    pub exit_code: Option<i32>,
    pub port_probe: Option<PortProbeStatus>,
}

pub fn probe_local_status(
    pid: Option<u32>,
    port_probe: Option<&PortProbeSpec>,
) -> Result<LocalFlowStatus, StatusError> {
    let running = pid.is_some_and(is_process_alive);
    let port_probe = if let Some(probe) = port_probe {
        Some(PortProbeStatus {
            port: probe.port,
            listening: tcp_port_listening(probe.port),
        })
    } else {
        None
    };

    let state = derive_local_flow_state(pid.is_some(), running, port_probe.as_ref());

    Ok(LocalFlowStatus {
        state,
        pid,
        running,
        exit_code: None,
        port_probe,
    })
}

fn derive_local_flow_state(
    has_pid: bool,
    running: bool,
    port_probe: Option<&PortProbeStatus>,
) -> LocalFlowState {
    if running {
        if port_probe.is_some_and(|probe| !probe.listening) {
            LocalFlowState::Starting
        } else {
            LocalFlowState::Running
        }
    } else if has_pid {
        LocalFlowState::Stopped
    } else {
        LocalFlowState::Unknown
    }
}

pub(crate) fn tcp_port_listening(port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&address, Duration::from_millis(150)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{
        derive_local_flow_state, probe_local_status, tcp_port_listening, LocalFlowState,
        PortProbeStatus,
    };
    use crate::spec::{PortProbeKind, PortProbeSpec};
    use std::net::TcpListener;

    #[test]
    fn probe_local_status_reports_unknown_without_pid() {
        let status = probe_local_status(None, None).expect("status probe should succeed");

        assert_eq!(status.state, LocalFlowState::Unknown);
        assert!(!status.running);
    }

    #[test]
    fn tcp_port_listening_detects_open_listener() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
        let port = listener.local_addr().expect("addr should exist").port();

        assert!(tcp_port_listening(port));
    }

    #[test]
    fn derive_local_flow_state_reports_starting_when_process_is_running_but_port_is_not_ready() {
        let probe = PortProbeStatus {
            port: 25565,
            listening: false,
        };

        let state = derive_local_flow_state(true, true, Some(&probe));

        assert_eq!(state, LocalFlowState::Starting);
    }

    #[test]
    fn probe_local_status_reports_stopped_for_missing_process_with_pid() {
        let probe = PortProbeSpec {
            port: 9,
            kind: PortProbeKind::TcpListening,
        };

        let status =
            probe_local_status(Some(u32::MAX), Some(&probe)).expect("status probe should succeed");

        assert_eq!(status.state, LocalFlowState::Stopped);
        assert!(!status.running);
    }
}
