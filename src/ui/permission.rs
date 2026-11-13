use agent_client_protocol::v1::schema::{PermissionOptionKind, RequestPermissionRequest, ToolKind};
use egui::{Color32, CornerRadius, Margin, RichText, Stroke};

use crate::theme;

pub enum PermissionAction {
    Select(String),
    Cancel,
}

pub fn draw(ctx: &egui::Context, request: &RequestPermissionRequest) -> Option<PermissionAction> {
    let mut action = None;
    let style = ctx.style_of(ctx.theme());
    let frame = egui::Frame::NONE
        .fill(style.visuals.window_fill)
        .stroke(Stroke::new(1.0, theme::border(style.visuals.dark_mode)))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(20));

    let response = egui::Modal::new(egui::Id::unique("permission_modal"))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_min_width((ctx.content_rect().width() - 72.0).clamp(300.0, 560.0));

            ui.horizontal(|ui| {
                ui.label(RichText::new("⚠").size(20.0).color(theme::WARNING));
                ui.label(RichText::new("Permission required").strong().size(18.0));
            });
            ui.add_space(8.0);

            let title = request
                .tool_call
                .fields
                .title
                .as_deref()
                .unwrap_or("Tool call");
            ui.label(RichText::new(title).strong().size(15.0));

            ui.horizontal(|ui| {
                if let Some(kind) = request.tool_call.fields.kind {
                    chip(ui, kind_label(kind), theme::INFO);
                }
                chip(
                    ui,
                    &format!("session {}", request.session_id),
                    theme::muted(ui),
                );
            });

            if let Some(input) = &request.tool_call.fields.raw_input {
                ui.add_space(10.0);
                egui::Frame::NONE
                    .fill(ui.visuals().extreme_bg_color)
                    .corner_radius(10)
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("permission_input")
                            .max_height(180.0)
                            .show(ui, |ui| {
                                let text = serde_json::to_string_pretty(input)
                                    .unwrap_or_else(|_| input.to_string());
                                ui.add(
                                    egui::Label::new(RichText::new(text).monospace())
                                        .selectable(true)
                                        .wrap(),
                                );
                            });
                    });
            }

            ui.add_space(14.0);
            for option in &request.options {
                let (fill, stroke, text) = match option.kind {
                    PermissionOptionKind::AllowOnce => {
                        (theme::ACCENT, Stroke::NONE, Color32::from_rgb(24, 20, 12))
                    }
                    PermissionOptionKind::AllowAlways => (
                        ui.visuals().faint_bg_color,
                        Stroke::new(1.0, theme::ACCENT_DIM),
                        ui.visuals().text_color(),
                    ),
                    PermissionOptionKind::RejectOnce => (
                        ui.visuals().faint_bg_color,
                        Stroke::new(1.0, theme::DANGER.gamma_multiply(0.7)),
                        theme::DANGER,
                    ),
                    PermissionOptionKind::RejectAlways => (
                        Color32::TRANSPARENT,
                        Stroke::new(1.0, theme::border(ui.visuals().dark_mode)),
                        theme::muted(ui),
                    ),
                    _ => (
                        ui.visuals().faint_bg_color,
                        Stroke::new(1.0, theme::border(ui.visuals().dark_mode)),
                        ui.visuals().text_color(),
                    ),
                };

                let button = egui::Button::new(RichText::new(&option.name).color(text).strong())
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(CornerRadius::same(9))
                    .min_size(egui::vec2(ui.available_width(), 36.0));

                if ui.add(button).clicked() {
                    action = Some(PermissionAction::Select(option.option_id.to_string()));
                }
                ui.add_space(6.0);
            }

            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    action = Some(PermissionAction::Cancel);
                }
                ui.label(
                    RichText::new("Esc to cancel")
                        .small()
                        .color(theme::muted(ui)),
                );
            });
        });

    if action.is_none() && response.should_close() {
        action = Some(PermissionAction::Cancel);
    }
    action
}

fn chip(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::NONE
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(20)
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).small().color(color));
        });
}

fn kind_label(kind: ToolKind) -> &'static str {
    match kind {
        ToolKind::Read => "read",
        ToolKind::Edit => "edit",
        ToolKind::Delete => "delete",
        ToolKind::Move => "move",
        ToolKind::Search => "search",
        ToolKind::Execute => "execute",
        ToolKind::Think => "think",
        ToolKind::Fetch => "fetch",
        ToolKind::SwitchMode => "switch mode",
        _ => "other",
    }
}
