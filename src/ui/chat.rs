use agent_client_protocol::v1::schema::{
    ContentBlock, ToolCall, ToolCallContent, ToolCallStatus, ToolKind,
};
use egui::{Color32, Margin, RichText, Stroke};
use egui_commonmark::CommonMarkCache;

use crate::branding::Logos;
use crate::diff;
use crate::markdown;
use crate::model::{SessionView, TranscriptItem};
use crate::theme;

pub fn draw(
    ui: &mut egui::Ui,
    session: &mut SessionView,
    cache: &mut CommonMarkCache,
    show_thoughts: bool,
    logos: &Logos,
) -> Option<String> {
    let session_id = session.id.clone();
    let title = session
        .title
        .clone()
        .unwrap_or_else(|| "New session".to_string());
    let cwd = session.cwd.display().to_string();

    ui.horizontal(|ui| {
        ui.label(RichText::new(title).strong().size(16.0));
        ui.add(egui::Label::new(RichText::new(cwd).small().color(theme::muted(ui))).truncate());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(usage) = &session.usage {
                usage_inline(ui, usage.used, usage.size);
            }
            if session.is_prompting {
                ui.spinner();
                ui.label(RichText::new("Working…").small().color(theme::accent()));
            }
        });
    });
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(8.0);

    if session.transcript.is_empty() {
        return welcome_body(ui, &session.cwd, true, logos);
    }

    let mut selection = session.selected_tool;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .content_margin(Margin {
            left: 0,
            right: 6,
            top: 0,
            bottom: 4,
        })
        .show(ui, |ui| {
            for (index, item) in session.transcript.iter_mut().enumerate() {
                match item {
                    TranscriptItem::User { text, .. } => {
                        ui.add_space(10.0);
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::TOP),
                            |ui| {
                                egui::Frame::NONE
                                    .fill(ui.visuals().selection.bg_fill)
                                    .stroke(Stroke::new(1.0, theme::accent_dim()))
                                    .corner_radius(12)
                                    .inner_margin(Margin::same(12))
                                    .show(ui, |ui| {
                                        ui.set_max_width(ui.available_width() * 0.78);
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(text.as_str())
                                                    .color(ui.visuals().strong_text_color()),
                                            )
                                            .selectable(true)
                                            .wrap(),
                                        );
                                    });
                            },
                        );
                        ui.add_space(10.0);
                    }
                    TranscriptItem::Assistant { text, .. } => {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("●")
                                    .size(8.0)
                                    .color(theme::accent()),
                            );
                            ui.label(
                                RichText::new("Command Code")
                                    .small()
                                    .strong()
                                    .color(theme::muted(ui)),
                            );
                        });
                        ui.add_space(2.0);
                        markdown::show(
                            ui,
                            cache,
                            egui::Id::unique(("assistant", session_id.as_str(), index)),
                            text,
                        );
                        ui.add_space(10.0);
                    }
                    TranscriptItem::Thought { text, open, .. } => {
                        if show_thoughts {
                            egui::Frame::NONE
                                .fill(ui.visuals().faint_bg_color)
                                .corner_radius(10)
                                .inner_margin(Margin::same(10))
                                .show(ui, |ui| {
                                    egui::CollapsingHeader::new(
                                        RichText::new("Thinking")
                                            .small()
                                            .italics()
                                            .color(theme::muted(ui)),
                                    )
                                    .id_salt(("thought", session_id.as_str(), index))
                                    .show_background(false)
                                    .default_open(*open)
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(text.as_str())
                                                    .italics()
                                                    .color(theme::muted(ui)),
                                            )
                                            .wrap(),
                                        );
                                    });
                                });
                            ui.add_space(8.0);
                        }
                    }
                    TranscriptItem::Tool { call, open } => {
                        draw_tool(
                            ui,
                            call,
                            open,
                            &session_id,
                            index,
                            &mut selection,
                        );
                        ui.add_space(8.0);
                    }
                    TranscriptItem::Plan { entries } => {
                        theme::card_frame(ui).show(ui, |ui| {
                            ui.label(
                                RichText::new("Plan")
                                    .small()
                                    .strong()
                                    .color(theme::accent()),
                            );
                            ui.add_space(4.0);
                            for entry in entries {
                                let (icon, color) = match entry.status {
                                    agent_client_protocol::v1::schema::PlanEntryStatus::Completed => {
                                        ("✔", theme::SUCCESS)
                                    }
                                    agent_client_protocol::v1::schema::PlanEntryStatus::InProgress => {
                                        ("◐", theme::WARNING)
                                    }
                                    agent_client_protocol::v1::schema::PlanEntryStatus::Pending => {
                                        ("○", theme::muted(ui))
                                    }
                                    _ => ("•", theme::muted(ui)),
                                };
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(icon).color(color));
                                    ui.label(RichText::new(entry.content.as_str()));
                                });
                            }
                        });
                        ui.add_space(8.0);
                    }
                    TranscriptItem::Notice { text } => {
                        egui::Frame::NONE
                            .fill(Color32::from_rgba_unmultiplied(126, 177, 214, 22))
                            .stroke(Stroke::new(
                                1.0,
                                theme::INFO.gamma_multiply(0.5),
                            ))
                            .corner_radius(10)
                            .inner_margin(Margin::same(10))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(text.as_str())
                                            .color(ui.visuals().strong_text_color()),
                                    )
                                    .wrap(),
                                );
                            });
                        ui.add_space(8.0);
                    }
                }
            }

            if session.is_prompting {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new("Working…")
                            .small()
                            .color(theme::muted(ui)),
                    );
                });
            }
            ui.add_space(6.0);
        });

    if let Some(index) = selection {
        session.selected_tool = Some(index);
    }
    None
}

pub fn welcome(ui: &mut egui::Ui, cwd: &std::path::Path, logos: &Logos) -> Option<String> {
    welcome_body(ui, cwd, false, logos)
}

fn usage_inline(ui: &mut egui::Ui, used: u64, size: u64) {
    let size = size.max(1);
    let ratio = (used as f32 / size as f32).clamp(0.0, 1.0);
    ui.label(
        RichText::new(format!("{}k / {}k", used / 1000, size / 1000))
            .small()
            .color(theme::muted(ui)),
    )
    .on_hover_text("context usage");
    let (rect, _) = ui.allocate_exact_size(egui::vec2(48.0, 4.0), egui::Sense::hover());
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(2),
        theme::border(ui.visuals().dark_mode),
    );
    let filled =
        egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * ratio, rect.height()));
    ui.painter()
        .rect_filled(filled, egui::CornerRadius::same(2), theme::accent());
    ui.add_space(4.0);
}

fn welcome_body(
    ui: &mut egui::Ui,
    cwd: &std::path::Path,
    in_session: bool,
    logos: &Logos,
) -> Option<String> {
    let mut suggestion = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(if in_session { 48.0 } else { 90.0 });
            ui.vertical_centered(|ui| {
                logos.show_wordmark(ui, 32.0).on_hover_text("Command Code");
                ui.add_space(12.0);
                ui.label(
                    RichText::new("What are we building?")
                        .strong()
                        .size(24.0),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(
                        "Command Code can read, edit, and run your project.",
                    )
                    .color(theme::muted(ui)),
                );
                ui.label(
                    RichText::new(cwd.display().to_string())
                        .small()
                        .color(theme::muted(ui)),
                );
                ui.add_space(24.0);

                let available = ui.available_width();
                let columns = if available >= 620.0 { 2 } else { 1 };
                let gap = 8.0;
                let button_width = if columns == 2 {
                    (available.min(720.0) - gap) / 2.0
                } else {
                    available.min(520.0)
                };
                let items = [
                    (
                        "Explain this codebase",
                        "Walk me through the architecture, the main entry points, and how the pieces fit together.",
                    ),
                    (
                        "Find and fix a bug",
                        "Look for likely bugs in this project, explain the most important one, and fix it.",
                    ),
                    (
                        "Write tests",
                        "Find untested code and add focused tests for the most important behavior.",
                    ),
                    (
                        "Review my changes",
                        "Review the current git diff for bugs, regressions, and missing tests.",
                    ),
                ];

                egui::Grid::new("welcome_suggestions")
                    .num_columns(columns)
                    .spacing([gap, gap])
                    .show(ui, |ui| {
                        for (index, (title, prompt)) in items.into_iter().enumerate() {
                            let button = egui::Button::new(
                                RichText::new(format!("{title}  ↗"))
                                    .color(ui.visuals().strong_text_color()),
                            )
                            .fill(theme::surface(ui))
                            .stroke(Stroke::new(
                                1.0,
                                theme::border(ui.visuals().dark_mode),
                            ))
                            .corner_radius(10)
                            .min_size(egui::vec2(button_width, 44.0));

                            if ui.add(button).clicked() {
                                suggestion = Some(prompt.to_string());
                            }
                            if (index + 1) % columns == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
        });

    suggestion
}

fn draw_tool(
    ui: &mut egui::Ui,
    call: &mut ToolCall,
    open: &mut bool,
    session_id: &str,
    index: usize,
    selection: &mut Option<usize>,
) {
    let (icon, color) = status_style(call.status);
    theme::card_frame(ui).show(ui, |ui| {
        ui.horizontal(|ui| {
            let button_width = 62.0;
            let content_width = (ui.available_width() - button_width).max(80.0);
            ui.allocate_ui_with_layout(
                egui::vec2(content_width, 22.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(RichText::new(icon).color(color).size(14.0));
                    ui.add(
                        egui::Label::new(RichText::new(call.title.as_str()).strong()).truncate(),
                    );
                    ui.label(
                        RichText::new(kind_label(call.kind))
                            .small()
                            .color(theme::muted(ui)),
                    );
                },
            );
            if ui.small_button("Details").clicked() {
                *selection = Some(index);
            }
        });

        egui::CollapsingHeader::new(
            RichText::new("Show details")
                .small()
                .color(theme::muted(ui)),
        )
        .id_salt(("tool", session_id, index))
        .show_background(false)
        .default_open(*open)
        .show(ui, |ui| {
            tool_body(ui, call);
        });
    });
}

pub fn tool_body(ui: &mut egui::Ui, call: &ToolCall) {
    ui.horizontal(|ui| {
        let (icon, color) = status_style(call.status);
        ui.colored_label(color, format!("{icon} {:?}", call.status).to_lowercase());
        if let Some(name) = &call.name {
            ui.label(
                RichText::new(name)
                    .monospace()
                    .small()
                    .color(theme::muted(ui)),
            );
        }
    });

    if !call.locations.is_empty() {
        ui.add_space(4.0);
        for location in &call.locations {
            match location.line {
                Some(line) => ui.label(
                    RichText::new(format!("{}:{line}", location.path.display()))
                        .monospace()
                        .small(),
                ),
                None => ui.label(
                    RichText::new(location.path.display().to_string())
                        .monospace()
                        .small(),
                ),
            };
        }
    }

    if !call.content.is_empty() {
        ui.add_space(6.0);
        for content in &call.content {
            match content {
                ToolCallContent::Content(content) => match &content.content {
                    ContentBlock::Text(text) => {
                        ui.add(
                            egui::Label::new(RichText::new(text.text.as_str()).monospace())
                                .selectable(true)
                                .wrap(),
                        );
                    }
                    ContentBlock::Image(_) => {
                        ui.label(RichText::new("[image]").italics());
                    }
                    _ => {
                        ui.label(RichText::new("[content]").italics());
                    }
                },
                ToolCallContent::Diff(diff) => {
                    ui.label(
                        RichText::new(diff.path.display().to_string())
                            .monospace()
                            .strong(),
                    );
                    diff::show_text_diff(ui, diff.old_text.as_deref(), &diff.new_text);
                }
                ToolCallContent::Terminal(terminal) => {
                    ui.label(format!("terminal: {}", terminal.terminal_id));
                }
                _ => {}
            }
        }
    }

    if let Some(input) = &call.raw_input {
        egui::CollapsingHeader::new(RichText::new("Input").small().color(theme::muted(ui)))
            .show_background(false)
            .show(ui, |ui| {
                diff::show_json(ui, input);
            });
    }
    if let Some(output) = &call.raw_output {
        egui::CollapsingHeader::new(RichText::new("Output").small().color(theme::muted(ui)))
            .show_background(false)
            .show(ui, |ui| {
                diff::show_json(ui, output);
            });
    }
}

pub fn status_style(status: ToolCallStatus) -> (&'static str, egui::Color32) {
    match status {
        ToolCallStatus::Completed => ("✔", theme::SUCCESS),
        ToolCallStatus::Failed => ("✖", theme::DANGER),
        ToolCallStatus::InProgress => ("◐", theme::WARNING),
        ToolCallStatus::Pending => ("○", theme::muted_color()),
        _ => ("•", theme::muted_color()),
    }
}

pub fn kind_label(kind: ToolKind) -> &'static str {
    match kind {
        ToolKind::Read => "read",
        ToolKind::Edit => "edit",
        ToolKind::Delete => "delete",
        ToolKind::Move => "move",
        ToolKind::Search => "search",
        ToolKind::Execute => "execute",
        ToolKind::Think => "think",
        ToolKind::Fetch => "fetch",
        ToolKind::SwitchMode => "mode",
        _ => "tool",
    }
}
