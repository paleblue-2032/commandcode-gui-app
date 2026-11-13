use std::thread;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::v1::ConnectionTo;
use agent_client_protocol::v1::schema::{
    CancelNotification, ContentBlock, Implementation, InitializeRequest, LoadSessionRequest,
    NewSessionRequest, PromptRequest, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SessionConfigOptionValue, SessionId, SessionNotification,
    SetSessionConfigOptionRequest, SetSessionModeRequest, TextContent,
};
use agent_client_protocol::{AcpAgent, AcpAgentConfig, Agent, Client};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::oneshot;

use super::events::{AcpCommand, AcpEvent, AgentConfig};

pub struct AcpHandle {
    pub tx: UnboundedSender<AcpCommand>,
    pub rx: UnboundedReceiver<AcpEvent>,
}

impl AcpHandle {
    pub fn shutdown(&self) {
        let _ = self.tx.send(AcpCommand::Shutdown);
    }
}

impl Drop for AcpHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn spawn(config: AgentConfig, ctx: egui::Context) -> AcpHandle {
    let (cmd_tx, cmd_rx) = unbounded_channel();
    let (evt_tx, evt_rx) = unbounded_channel();

    thread::Builder::new()
        .name("acp-worker".to_string())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            match runtime {
                Ok(runtime) => runtime.block_on(run(config, cmd_rx, evt_tx, ctx)),
                Err(error) => {
                    let _ = evt_tx.send(AcpEvent::Error {
                        message: format!("failed to start ACP runtime: {error}"),
                    });
                    ctx.request_repaint();
                }
            }
        })
        .ok();

    AcpHandle {
        tx: cmd_tx,
        rx: evt_rx,
    }
}

async fn run(
    config: AgentConfig,
    mut cmd_rx: UnboundedReceiver<AcpCommand>,
    evt_tx: UnboundedSender<AcpEvent>,
    ctx: egui::Context,
) {
    let agent = AcpAgent::new(AcpAgentConfig::new(&config.binary).args(config.args));

    let notification_tx = evt_tx.clone();
    let notification_ctx = ctx.clone();
    let permission_tx = evt_tx.clone();
    let permission_ctx = ctx.clone();
    let loop_tx = evt_tx.clone();
    let loop_ctx = ctx.clone();

    let result = Client
        .v1()
        .name("commandcode-gui")
        .on_receive_notification(
            async move |notification: SessionNotification, _cx| {
                send(
                    &notification_tx,
                    &notification_ctx,
                    AcpEvent::Update {
                        session_id: notification.session_id.to_string(),
                        update: notification.update,
                    },
                );
                Ok::<(), agent_client_protocol::Error>(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async move |request: RequestPermissionRequest, responder, _connection| {
                let (reply_tx, reply_rx) = oneshot::channel();
                send(
                    &permission_tx,
                    &permission_ctx,
                    AcpEvent::PermissionRequest {
                        request,
                        reply: reply_tx,
                    },
                );
                let response = reply_rx.await.unwrap_or_else(|_| {
                    RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled)
                });
                responder.respond(response)
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(agent, move |cx: ConnectionTo<Agent>| async move {
            let init_response = cx
                .send_request(InitializeRequest::new(ProtocolVersion::V1).client_info(
                    Implementation::new("commandcode-gui", env!("CARGO_PKG_VERSION")),
                ))
                .block_task()
                .await?;

            let agent_info = init_response.agent_info.as_ref();
            send(
                &loop_tx,
                &loop_ctx,
                AcpEvent::Connected {
                    agent_name: agent_info
                        .map(|info| info.name.clone())
                        .unwrap_or_else(|| "Command Code".to_string()),
                    agent_version: agent_info
                        .map(|info| info.version.clone())
                        .unwrap_or_default(),
                    protocol_version: format!("ACP v{}", init_response.protocol_version.as_u16()),
                    auth_methods: init_response
                        .auth_methods
                        .iter()
                        .map(|method| method.name().to_string())
                        .collect(),
                },
            );

            while let Some(command) = cmd_rx.recv().await {
                match command {
                    AcpCommand::NewSession { cwd } => {
                        match cx
                            .send_request(NewSessionRequest::new(cwd.clone()))
                            .block_task()
                            .await
                        {
                            Ok(response) => {
                                send(
                                    &loop_tx,
                                    &loop_ctx,
                                    AcpEvent::SessionReady {
                                        session_id: response.session_id.to_string(),
                                        cwd,
                                        modes: response.modes,
                                        config_options: response.config_options.unwrap_or_default(),
                                        loaded: false,
                                    },
                                );
                            }
                            Err(error) => {
                                send(
                                    &loop_tx,
                                    &loop_ctx,
                                    AcpEvent::Error {
                                        message: format!("session/new failed: {error}"),
                                    },
                                );
                            }
                        }
                    }
                    AcpCommand::LoadSession { session_id, cwd } => {
                        send(
                            &loop_tx,
                            &loop_ctx,
                            AcpEvent::SessionLoading {
                                session_id: session_id.clone(),
                                cwd: cwd.clone(),
                            },
                        );
                        match cx
                            .send_request(LoadSessionRequest::new(
                                SessionId::new(session_id.clone()),
                                cwd.clone(),
                            ))
                            .block_task()
                            .await
                        {
                            Ok(response) => {
                                send(
                                    &loop_tx,
                                    &loop_ctx,
                                    AcpEvent::SessionReady {
                                        session_id,
                                        cwd,
                                        modes: response.modes,
                                        config_options: response.config_options.unwrap_or_default(),
                                        loaded: true,
                                    },
                                );
                            }
                            Err(error) => {
                                send(
                                    &loop_tx,
                                    &loop_ctx,
                                    AcpEvent::Error {
                                        message: format!("session/load failed: {error}"),
                                    },
                                );
                            }
                        }
                    }
                    AcpCommand::Prompt { session_id, text } => {
                        let prompt_cx = cx.clone();
                        let prompt_tx = loop_tx.clone();
                        let prompt_ctx = loop_ctx.clone();
                        let prompt_session = session_id.clone();
                        let spawned = cx.spawn(async move {
                            let request = PromptRequest::new(
                                SessionId::new(prompt_session.clone()),
                                vec![ContentBlock::Text(TextContent::new(text))],
                            );
                            match prompt_cx.send_request(request).block_task().await {
                                Ok(response) => send(
                                    &prompt_tx,
                                    &prompt_ctx,
                                    AcpEvent::PromptFinished {
                                        session_id: prompt_session,
                                        stop_reason: response.stop_reason,
                                    },
                                ),
                                Err(error) => send(
                                    &prompt_tx,
                                    &prompt_ctx,
                                    AcpEvent::Error {
                                        message: format!("session/prompt failed: {error}"),
                                    },
                                ),
                            }
                            Ok::<(), agent_client_protocol::Error>(())
                        });
                        if let Err(error) = spawned {
                            send(
                                &loop_tx,
                                &loop_ctx,
                                AcpEvent::Error {
                                    message: format!("failed to spawn prompt: {error}"),
                                },
                            );
                        }
                    }
                    AcpCommand::Cancel { session_id } => {
                        if let Err(error) = cx
                            .send_notification(CancelNotification::new(SessionId::new(session_id)))
                        {
                            send(
                                &loop_tx,
                                &loop_ctx,
                                AcpEvent::Error {
                                    message: format!("session/cancel failed: {error}"),
                                },
                            );
                        }
                    }
                    AcpCommand::SetMode {
                        session_id,
                        mode_id,
                    } => {
                        if let Err(error) = cx
                            .send_request(SetSessionModeRequest::new(
                                SessionId::new(session_id),
                                mode_id,
                            ))
                            .block_task()
                            .await
                        {
                            send(
                                &loop_tx,
                                &loop_ctx,
                                AcpEvent::Error {
                                    message: format!("session/set_mode failed: {error}"),
                                },
                            );
                        }
                    }
                    AcpCommand::SetConfigOption {
                        session_id,
                        config_id,
                        value,
                    } => {
                        let request = SetSessionConfigOptionRequest::new(
                            SessionId::new(session_id.clone()),
                            config_id,
                            SessionConfigOptionValue::value_id(value),
                        );
                        match cx.send_request(request).block_task().await {
                            Ok(response) => {
                                send(
                                    &loop_tx,
                                    &loop_ctx,
                                    AcpEvent::ConfigOptionsChanged {
                                        session_id,
                                        config_options: response.config_options,
                                    },
                                );
                            }
                            Err(error) => {
                                send(
                                    &loop_tx,
                                    &loop_ctx,
                                    AcpEvent::Error {
                                        message: format!(
                                            "session/set_config_option failed: {error}"
                                        ),
                                    },
                                );
                            }
                        }
                    }
                    AcpCommand::Shutdown => break,
                }
            }

            Ok::<(), agent_client_protocol::Error>(())
        })
        .await;

    if let Err(error) = result {
        send(
            &evt_tx,
            &ctx,
            AcpEvent::Error {
                message: error.to_string(),
            },
        );
        send(
            &evt_tx,
            &ctx,
            AcpEvent::Closed {
                reason: error.to_string(),
            },
        );
    } else {
        send(
            &evt_tx,
            &ctx,
            AcpEvent::Closed {
                reason: "connection closed".to_string(),
            },
        );
    }
}

fn send(tx: &UnboundedSender<AcpEvent>, ctx: &egui::Context, event: AcpEvent) {
    if tx.send(event).is_ok() {
        ctx.request_repaint();
    }
}
