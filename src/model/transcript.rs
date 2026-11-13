use std::path::PathBuf;

use agent_client_protocol::v1::schema::{
    AvailableCommand, ContentBlock, PlanEntry, SessionConfigOption, SessionModeState,
    SessionUpdate, StopReason, ToolCall, UsageUpdate,
};

pub struct SessionView {
    pub id: String,
    pub cwd: PathBuf,
    pub title: Option<String>,
    pub modes: Option<SessionModeState>,
    pub config_options: Vec<SessionConfigOption>,
    pub transcript: Vec<TranscriptItem>,
    pub current_assistant: Option<usize>,
    pub current_thought: Option<usize>,
    pub is_prompting: bool,
    pub stop_reason: Option<StopReason>,
    pub available_commands: Vec<AvailableCommand>,
    pub usage: Option<UsageUpdate>,
    pub loading_history: bool,
    pub selected_tool: Option<usize>,
}

pub enum TranscriptItem {
    User {
        text: String,
        message_id: Option<String>,
    },
    Assistant {
        text: String,
        message_id: Option<String>,
    },
    Thought {
        text: String,
        message_id: Option<String>,
        open: bool,
    },
    Tool {
        call: Box<ToolCall>,
        open: bool,
    },
    Plan {
        entries: Vec<PlanEntry>,
    },
    Notice {
        text: String,
    },
}

impl SessionView {
    pub fn new(id: String, cwd: PathBuf) -> Self {
        Self {
            id,
            cwd,
            title: None,
            modes: None,
            config_options: Vec::new(),
            transcript: Vec::new(),
            current_assistant: None,
            current_thought: None,
            is_prompting: false,
            stop_reason: None,
            available_commands: Vec::new(),
            usage: None,
            loading_history: false,
            selected_tool: None,
        }
    }

    pub fn push_user(&mut self, text: String) {
        self.transcript.push(TranscriptItem::User {
            text,
            message_id: None,
        });
        self.current_assistant = None;
        self.current_thought = None;
    }

    pub fn find_tool(&self, tool_call_id: &str) -> Option<usize> {
        self.transcript.iter().position(|item| {
            matches!(
                item,
                TranscriptItem::Tool { call, .. } if call.tool_call_id.to_string() == tool_call_id
            )
        })
    }

    pub fn apply_update(&mut self, update: SessionUpdate) {
        match update {
            SessionUpdate::UserMessageChunk(chunk) if self.loading_history => {
                if let Some(text) = content_text(&chunk.content) {
                    let message_id = chunk.message_id.map(|id| id.to_string());
                    match self.transcript.last_mut() {
                        Some(TranscriptItem::User {
                            text: last_text,
                            message_id: last_id,
                        }) if last_id == &message_id
                            || last_id.is_none()
                            || message_id.is_none() =>
                        {
                            last_text.push_str(&text);
                        }
                        _ => {
                            self.transcript
                                .push(TranscriptItem::User { text, message_id });
                        }
                    }
                }
            }
            SessionUpdate::AgentMessageChunk(chunk) => {
                if let Some(text) = content_text(&chunk.content) {
                    let message_id = chunk.message_id.map(|id| id.to_string());
                    if let Some(index) = self.current_assistant
                        && let Some(TranscriptItem::Assistant {
                            text: last_text,
                            message_id: last_id,
                        }) = self.transcript.get_mut(index)
                        && (last_id == &message_id || last_id.is_none() || message_id.is_none())
                    {
                        last_text.push_str(&text);
                        return;
                    }
                    self.transcript
                        .push(TranscriptItem::Assistant { text, message_id });
                    self.current_assistant = Some(self.transcript.len() - 1);
                    self.current_thought = None;
                }
            }
            SessionUpdate::AgentThoughtChunk(chunk) => {
                if let Some(text) = content_text(&chunk.content) {
                    let message_id = chunk.message_id.map(|id| id.to_string());
                    if let Some(index) = self.current_thought
                        && let Some(TranscriptItem::Thought {
                            text: last_text,
                            message_id: last_id,
                            ..
                        }) = self.transcript.get_mut(index)
                        && (last_id == &message_id || last_id.is_none() || message_id.is_none())
                    {
                        last_text.push_str(&text);
                        return;
                    }
                    self.transcript.push(TranscriptItem::Thought {
                        text,
                        message_id,
                        open: false,
                    });
                    self.current_thought = Some(self.transcript.len() - 1);
                    self.current_assistant = None;
                }
            }
            SessionUpdate::ToolCall(call) => {
                self.transcript.push(TranscriptItem::Tool {
                    call: Box::new(call),
                    open: false,
                });
                self.current_assistant = None;
                self.current_thought = None;
            }
            SessionUpdate::ToolCallUpdate(update) => {
                let id = update.tool_call_id.to_string();
                if let Some(index) = self.find_tool(&id)
                    && let Some(TranscriptItem::Tool { call, .. }) = self.transcript.get_mut(index)
                {
                    call.update(update.fields);
                } else {
                    let title = update
                        .fields
                        .title
                        .clone()
                        .unwrap_or_else(|| "Tool call".to_string());
                    let mut call = ToolCall::new(update.tool_call_id.clone(), title);
                    call.update(update.fields);
                    self.transcript.push(TranscriptItem::Tool {
                        call: Box::new(call),
                        open: false,
                    });
                }
                self.current_assistant = None;
                self.current_thought = None;
            }
            SessionUpdate::Plan(plan) => {
                if let Some(TranscriptItem::Plan { entries }) = self
                    .transcript
                    .iter_mut()
                    .rev()
                    .find(|item| matches!(item, TranscriptItem::Plan { .. }))
                {
                    *entries = plan.entries;
                } else {
                    self.transcript.push(TranscriptItem::Plan {
                        entries: plan.entries,
                    });
                }
            }
            SessionUpdate::AvailableCommandsUpdate(update) => {
                self.available_commands = update.available_commands;
            }
            SessionUpdate::CurrentModeUpdate(update) => {
                if let Some(modes) = &mut self.modes {
                    modes.current_mode_id = update.current_mode_id;
                }
            }
            SessionUpdate::ConfigOptionUpdate(update) => {
                self.config_options = update.config_options;
            }
            SessionUpdate::SessionInfoUpdate(update) => {
                if let Some(title) = update.title.take() {
                    self.title = Some(title);
                }
            }
            SessionUpdate::UsageUpdate(update) => {
                self.usage = Some(update);
            }
            SessionUpdate::Notice(notice) => {
                let mut text = notice.title;
                if let Some(description) = notice.description {
                    text.push_str("\n\n");
                    text.push_str(&description);
                }
                self.transcript.push(TranscriptItem::Notice { text });
            }
            SessionUpdate::CompactionUpdate(_) | SessionUpdate::CompactionSummaryChunk(_) => {}
            _ => {}
        }
    }
}

fn content_text(block: &ContentBlock) -> Option<String> {
    match block {
        ContentBlock::Text(text) => Some(text.text.clone()),
        ContentBlock::Image(_) => Some("[image]".to_string()),
        ContentBlock::Audio(_) => Some("[audio]".to_string()),
        ContentBlock::ResourceLink(_) => Some("[resource link]".to_string()),
        ContentBlock::Resource(_) => Some("[resource]".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::v1::schema::{
        ContentChunk, TextContent, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
    };

    fn chunk(text: &str) -> ContentChunk {
        ContentChunk::new(ContentBlock::Text(TextContent::new(text)))
    }

    #[test]
    fn merges_streamed_assistant_text() {
        let mut session = SessionView::new("s".to_string(), PathBuf::from("/tmp"));
        session.apply_update(SessionUpdate::AgentMessageChunk(chunk("hel")));
        session.apply_update(SessionUpdate::AgentMessageChunk(chunk("lo")));

        match &session.transcript[0] {
            TranscriptItem::Assistant { text, .. } => assert_eq!(text, "hello"),
            _ => panic!("expected assistant item"),
        }
    }

    #[test]
    fn applies_tool_call_updates() {
        let mut session = SessionView::new("s".to_string(), PathBuf::from("/tmp"));
        session.apply_update(SessionUpdate::ToolCall(ToolCall::new("t1", "run")));
        session.apply_update(SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
            "t1",
            ToolCallUpdateFields::new().status(ToolCallStatus::Completed),
        )));

        match &session.transcript[0] {
            TranscriptItem::Tool { call, .. } => {
                assert_eq!(call.status, ToolCallStatus::Completed);
            }
            _ => panic!("expected tool item"),
        }
    }
}
