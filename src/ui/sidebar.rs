use std::path::{Path, PathBuf};
use std::time::SystemTime;

use egui::{CornerRadius, Margin, RichText, Stroke};

use crate::catalog::SessionSummary;
use crate::theme;

pub enum SidebarAction {
    None,
    NewSession,
    Refresh,
    Load(SessionSummary),
}

pub fn draw(
    ui: &mut egui::Ui,
    sessions: &[SessionSummary],
    filter: &mut String,
    active_id: Option<&str>,
    cwd: &Path,
) -> SidebarAction {
    let mut action = SidebarAction::None;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("SESSIONS")
                .small()
                .strong()
                .color(theme::muted(ui)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(RichText::new("+").size(16.0)).frame(false))
                .on_hover_text("New session")
                .clicked()
            {
                action = SidebarAction::NewSession;
            }
            if ui
                .add(egui::Button::new(RichText::new("⟳").size(14.0)).frame(false))
                .on_hover_text("Refresh sessions")
                .clicked()
            {
                action = SidebarAction::Refresh;
            }
        });
    });
    ui.label(
        RichText::new(short_path(cwd))
            .small()
            .color(theme::muted(ui)),
    )
    .on_hover_text(cwd.display().to_string());
    ui.add_space(4.0);

    egui::Frame::NONE
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(8)
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(filter)
                    .frame(egui::Frame::NONE)
                    .hint_text("Search sessions")
                    .desired_width(f32::INFINITY),
            );
        });

    ui.add_space(4.0);

    let filter_lower = filter.to_lowercase();
    let filtered: Vec<&SessionSummary> = sessions
        .iter()
        .filter(|session| {
            filter_lower.is_empty()
                || session.title.to_lowercase().contains(&filter_lower)
                || session
                    .cwd
                    .display()
                    .to_string()
                    .to_lowercase()
                    .contains(&filter_lower)
        })
        .collect();

    if filtered.is_empty() {
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(if sessions.is_empty() {
                    "No sessions yet"
                } else {
                    "No matching sessions"
                })
                .color(theme::muted(ui)),
            );
            ui.add_space(4.0);
            if ui.button("Start a session").clicked() {
                action = SidebarAction::NewSession;
            }
        });
        return action;
    }

    let mut groups: Vec<(PathBuf, Vec<&SessionSummary>)> = Vec::new();
    for session in filtered {
        if let Some((_, group)) = groups
            .iter_mut()
            .find(|(group_cwd, _)| group_cwd == &session.cwd)
        {
            group.push(session);
        } else {
            groups.push((session.cwd.clone(), vec![session]));
        }
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (group_cwd, group) in groups {
                let label = format!("{}  ({})", project_label(&group_cwd), group.len());
                egui::CollapsingHeader::new(
                    RichText::new(label)
                        .small()
                        .strong()
                        .color(theme::muted(ui)),
                )
                .id_salt(&group_cwd)
                .show_background(false)
                .default_open(true)
                .show(ui, |ui| {
                    for session in group {
                        let selected = active_id == Some(session.id.as_str());
                        if draw_session_row(ui, session, selected, theme::muted(ui)) {
                            action = SidebarAction::Load(session.clone());
                        }
                    }
                });
            }
        });

    action
}

fn draw_session_row(
    ui: &mut egui::Ui,
    session: &SessionSummary,
    selected: bool,
    muted: egui::Color32,
) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 48.0), egui::Sense::click());
    let response = response.on_hover_text(format!(
        "id: {}\nupdated: {}\npath: {}",
        session.id,
        format_time(session.updated),
        session.path.display()
    ));

    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        let fill = if selected {
            ui.visuals().selection.bg_fill
        } else if hovered {
            ui.visuals().widgets.hovered.weak_bg_fill
        } else {
            egui::Color32::TRANSPARENT
        };
        let stroke = if selected {
            Stroke::new(1.0, theme::ACCENT_DIM)
        } else {
            Stroke::NONE
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(8),
            fill,
            stroke,
            egui::StrokeKind::Inside,
        );

        if selected {
            let bar = egui::Rect::from_min_size(
                rect.left_top() + egui::vec2(0.0, 10.0),
                egui::vec2(3.0, rect.height() - 20.0),
            );
            ui.painter()
                .rect_filled(bar, CornerRadius::same(2), theme::ACCENT);
        }

        let inner = rect.shrink2(egui::vec2(12.0, 6.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        child.vertical(|ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(session.title.as_str())
                        .strong()
                        .size(13.0)
                        .color(ui.visuals().strong_text_color()),
                )
                .truncate(),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(format!(
                        "{}  ·  {}",
                        format_time(session.updated),
                        short_id(&session.id)
                    ))
                    .small()
                    .color(muted),
                )
                .truncate(),
            );
        });
    }

    response.clicked()
}

fn format_time(updated: SystemTime) -> String {
    let datetime: chrono::DateTime<chrono::Utc> = updated.into();
    datetime
        .with_timezone(&chrono::Local)
        .format("%m-%d %H:%M")
        .to_string()
}

fn project_label(cwd: &Path) -> String {
    cwd.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project")
        .to_string()
}

fn short_path(path: &Path) -> String {
    project_label(path)
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}
