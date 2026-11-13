use std::time::{Duration, Instant};

use agent_client_protocol::v1::schema::{
    ContentBlock, PermissionOptionKind, RequestPermissionOutcome, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionUpdate,
};
use commandcode_gui_app::acp::{self, AcpCommand, AcpEvent, AgentConfig};

fn main() -> anyhow::Result<()> {
    let ctx = egui::Context::default();
    let mut handle = acp::spawn(AgentConfig::default(), ctx);
    let start = Instant::now();
    let mut stage = "connect";
    let mut response = String::new();

    loop {
        if start.elapsed() > Duration::from_secs(120) {
            anyhow::bail!("timed out in stage {stage}");
        }

        while let Ok(event) = handle.rx.try_recv() {
            match event {
                AcpEvent::Connected {
                    agent_name,
                    agent_version,
                    protocol_version,
                    ..
                } => {
                    println!("connected: {agent_name} {agent_version} ({protocol_version})");
                    stage = "new-session";
                    handle.tx.send(AcpCommand::NewSession {
                        cwd: std::env::current_dir()?,
                    })?;
                }
                AcpEvent::SessionReady {
                    session_id: id,
                    loaded,
                    ..
                } => {
                    println!("session ready: {id} (loaded={loaded})");
                    stage = "prompt";
                    handle.tx.send(AcpCommand::Prompt {
                        session_id: id,
                        text: "Reply with exactly: pong".to_string(),
                    })?;
                }
                AcpEvent::Update { update, .. } => {
                    if let SessionUpdate::AgentMessageChunk(chunk) = update
                        && let ContentBlock::Text(text) = chunk.content
                    {
                        print!("{}", text.text);
                        response.push_str(&text.text);
                    }
                }
                AcpEvent::PermissionRequest { request, reply } => {
                    println!(
                        "permission requested for {:?}",
                        request.tool_call.fields.title
                    );
                    let option = request.options.iter().find(|option| {
                        matches!(
                            option.kind,
                            PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
                        )
                    });
                    let response = match option {
                        Some(option) => {
                            RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
                                SelectedPermissionOutcome::new(option.option_id.clone()),
                            ))
                        }
                        None => RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled),
                    };
                    let _ = reply.send(response);
                }
                AcpEvent::PromptFinished { stop_reason, .. } => {
                    println!("\nturn finished: {stop_reason:?}");
                    if !response.contains("pong") {
                        anyhow::bail!("unexpected response: {response:?}");
                    }
                    return Ok(());
                }
                AcpEvent::Error { message } => {
                    eprintln!("error: {message}");
                }
                AcpEvent::Closed { reason } => {
                    anyhow::bail!("connection closed: {reason}");
                }
                AcpEvent::SessionLoading { .. } | AcpEvent::ConfigOptionsChanged { .. } => {}
            }
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}
