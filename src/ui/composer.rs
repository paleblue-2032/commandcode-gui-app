use egui::{CornerRadius, Margin, RichText, Stroke};

use crate::theme;

pub enum ComposerAction {
    None,
    Send,
    Cancel,
}

pub fn draw(
    ui: &mut egui::Ui,
    text: &mut String,
    is_prompting: bool,
    has_session: bool,
    stop_reason: Option<&str>,
    request_focus: bool,
) -> ComposerAction {
    let mut action = ComposerAction::None;
    let narrow = ui.available_width() < 520.0;

    egui::Frame::NONE
        .fill(theme::surface(ui))
        .stroke(Stroke::new(1.0, theme::border(ui.visuals().dark_mode)))
        .corner_radius(12)
        .inner_margin(Margin {
            left: 12,
            right: 12,
            top: 8,
            bottom: 8,
        })
        .show(ui, |ui| {
            let edit = ui.add(
                egui::TextEdit::multiline(text)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .frame(egui::Frame::NONE)
                    .hint_text("Ask Command Code…"),
            );
            if request_focus {
                edit.request_focus();
            }

            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let enter_pressed = edit.has_focus()
                    && ui.input(|input| {
                        input.key_pressed(egui::Key::Enter)
                            && !input.modifiers.shift
                            && !input
                                .events
                                .iter()
                                .any(|event| matches!(event, egui::Event::Ime(_)))
                    });

                if enter_pressed && has_session && !is_prompting && !text.trim().is_empty() {
                    action = ComposerAction::Send;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if is_prompting {
                        let button =
                            egui::Button::new(RichText::new("Stop").color(theme::DANGER).strong())
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(Stroke::new(1.0, theme::DANGER.gamma_multiply(0.6)))
                                .corner_radius(CornerRadius::same(8))
                                .min_size(egui::vec2(72.0, 30.0));
                        if ui.add(button).clicked() {
                            action = ComposerAction::Cancel;
                        }
                        ui.spinner();
                    } else {
                        let enabled = has_session && !text.trim().is_empty();
                        let button = egui::Button::new(
                            RichText::new("Send  ↵")
                                .strong()
                                .color(theme::accent_text()),
                        )
                        .fill(theme::accent())
                        .stroke(Stroke::NONE)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(egui::vec2(88.0, 30.0));
                        if ui.add_enabled(enabled, button).clicked() {
                            action = ComposerAction::Send;
                        }
                    }

                    let hint = if is_prompting {
                        "Esc to stop"
                    } else if narrow {
                        "Enter to send"
                    } else {
                        "Enter to send · Shift+Enter for newline"
                    };
                    ui.label(RichText::new(hint).small().color(theme::muted(ui)));
                });
            });

            if let Some(reason) = stop_reason {
                ui.add_space(2.0);
                ui.label(
                    RichText::new(format!("last turn: {reason}"))
                        .small()
                        .color(theme::muted(ui)),
                );
            }
        });

    action
}
