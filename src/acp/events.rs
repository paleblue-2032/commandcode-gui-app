use std::path::PathBuf;

use agent_client_protocol::v1::schema::{
    RequestPermissionRequest, RequestPermissionResponse, SessionConfigOption, SessionModeState,
    SessionUpdate, StopReason,
};
use tokio::sync::oneshot;

#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub binary: String,
    pub args: Vec<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            binary: "commandcode".to_string(),
            args: vec!["--no-auto-update".to_string(), "acp".to_string()],
        }
    }
}

#[derive(Debug)]
pub enum AcpCommand {
    NewSession {
        cwd: PathBuf,
    },
    LoadSession {
        session_id: String,
        cwd: PathBuf,
    },
    Prompt {
        session_id: String,
        text: String,
    },
    Cancel {
        session_id: String,
    },
    SetMode {
        session_id: String,
        mode_id: String,
    },
    SetConfigOption {
        session_id: String,
        config_id: String,
        value: String,
    },
    Shutdown,
}

pub enum AcpEvent {
    Connected {
        agent_name: String,
        agent_version: String,
        protocol_version: String,
        auth_methods: Vec<String>,
    },
    SessionLoading {
        session_id: String,
        cwd: PathBuf,
    },
    SessionReady {
        session_id: String,
        cwd: PathBuf,
        modes: Option<SessionModeState>,
        config_options: Vec<SessionConfigOption>,
        loaded: bool,
    },
    Update {
        session_id: String,
        update: SessionUpdate,
    },
    PromptFinished {
        session_id: String,
        stop_reason: StopReason,
    },
    PermissionRequest {
        request: RequestPermissionRequest,
        reply: oneshot::Sender<RequestPermissionResponse>,
    },
    ConfigOptionsChanged {
        session_id: String,
        config_options: Vec<SessionConfigOption>,
    },
    Error {
        message: String,
    },
    Closed {
        reason: String,
    },
}
