use egui::{CornerRadius, Margin, RichText, Stroke};

use crate::settings::{PermissionPolicy, Settings, ThemePreference};
use crate::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsAction {
    None,
    Save,
    Cancel,
    Reconnect,
}

pub fn draw(
    ctx: &egui::Context,
    settings: &mut Settings,
    args_text: &mut String,
    default_cwd_text: &mut String,
) -> SettingsAction {
    let mut action = SettingsAction::None;
    let modal_width = (ctx.content_rect().width() - 72.0).clamp(320.0, 680.0);
    let field_width = (modal_width - 140.0).clamp(150.0, 420.0);
    let style = ctx.style_of(ctx.theme());
    let frame = egui::Frame::NONE
        .fill(style.visuals.window_fill)
        .stroke(Stroke::new(1.0, theme::border(style.visuals.dark_mode)))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(20));

    let response = egui::Modal::new(egui::Id::unique("settings_modal"))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_min_width(modal_width);

            ui.label(RichText::new("Settings").strong().size(20.0));
            ui.label(
                RichText::new("Agent, appearance, and permission behavior.")
                    .small()
                    .color(theme::muted(ui)),
            );
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .max_height(520.0)
                .show(ui, |ui| {
                    section(ui, "Agent");
                    egui::Grid::new("settings_agent")
                        .num_columns(2)
                        .spacing([14.0, 10.0])
                        .show(ui, |ui| {
                            field_label(ui, "Binary");
                            ui.add(
                                egui::TextEdit::singleline(&mut settings.agent_binary)
                                    .desired_width(field_width),
                            );
                            ui.end_row();

                            field_label(ui, "Arguments");
                            ui.add(
                                egui::TextEdit::singleline(args_text)
                                    .hint_text("--no-auto-update acp")
                                    .desired_width(field_width),
                            );
                            ui.end_row();

                            field_label(ui, "Default project");
                            ui.add(
                                egui::TextEdit::singleline(default_cwd_text)
                                    .hint_text("(empty = launch directory)")
                                    .desired_width(field_width),
                            );
                            ui.end_row();
                        });

                    ui.add_space(14.0);
                    section(ui, "Appearance");
                    egui::Grid::new("settings_appearance")
                        .num_columns(2)
                        .spacing([14.0, 10.0])
                        .show(ui, |ui| {
                            field_label(ui, "Theme");
                            ui.horizontal_wrapped(|ui| {
                                ui.selectable_value(
                                    &mut settings.theme,
                                    ThemePreference::System,
                                    "System",
                                );
                                ui.selectable_value(
                                    &mut settings.theme,
                                    ThemePreference::Dark,
                                    "Dark",
                                );
                                ui.selectable_value(
                                    &mut settings.theme,
                                    ThemePreference::Light,
                                    "Light",
                                );
                            });
                            ui.end_row();

                            field_label(ui, "Accent");
                            ui.horizontal_wrapped(|ui| {
                                for (name, rgb) in ACCENT_PRESETS {
                                    let response =
                                        color_swatch(ui, *rgb, settings.accent_color == *rgb)
                                            .on_hover_text(*name);
                                    if response.clicked() {
                                        settings.accent_color = *rgb;
                                    }
                                }
                                ui.color_edit_button_srgb(&mut settings.accent_color)
                                    .on_hover_text("Custom accent color");
                            });
                            ui.end_row();

                            field_label(ui, "Font scale");
                            ui.add(
                                egui::Slider::new(&mut settings.font_scale, 0.75..=1.5)
                                    .step_by(0.05),
                            );
                            ui.end_row();

                            field_label(ui, "Thinking");
                            ui.checkbox(&mut settings.show_thoughts, "Show thinking chunks");
                            ui.end_row();
                        });

                    ui.add_space(14.0);
                    section(ui, "Permissions");
                    ui.label(
                        RichText::new(
                            "Ask every time keeps the approval dialog for every tool call.",
                        )
                        .small()
                        .color(theme::muted(ui)),
                    );
                    ui.add_space(4.0);
                    egui::ComboBox::from_id_salt("permission_policy")
                        .selected_text(permission_label(settings.permission_policy))
                        .width(260.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut settings.permission_policy,
                                PermissionPolicy::Ask,
                                "Ask every time",
                            );
                            ui.selectable_value(
                                &mut settings.permission_policy,
                                PermissionPolicy::AutoAllowReads,
                                "Auto-allow reads",
                            );
                            ui.selectable_value(
                                &mut settings.permission_policy,
                                PermissionPolicy::Yolo,
                                "YOLO (allow all)",
                            );
                        });

                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(
                            "Binary or argument changes apply after reconnecting the agent.",
                        )
                        .small()
                        .color(theme::muted(ui)),
                    );
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let save = egui::Button::new(
                    RichText::new("Save changes")
                        .strong()
                        .color(theme::accent_text()),
                )
                .fill(theme::accent())
                .stroke(Stroke::NONE)
                .corner_radius(CornerRadius::same(8))
                .min_size(egui::vec2(120.0, 32.0));
                if ui.add(save).clicked() {
                    action = SettingsAction::Save;
                }
                if ui.button("Cancel").clicked() {
                    action = SettingsAction::Cancel;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Reconnect agent").clicked() {
                        action = SettingsAction::Reconnect;
                    }
                });
            });
        });

    if action == SettingsAction::None && response.should_close() {
        action = SettingsAction::Cancel;
    }
    action
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.label(
        RichText::new(title.to_uppercase())
            .small()
            .strong()
            .color(theme::accent()),
    );
    ui.add_space(4.0);
}

fn color_swatch(ui: &mut egui::Ui, rgb: [u8; 3], selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
    let color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    ui.painter().circle_filled(rect.center(), 8.5, color);
    if selected {
        ui.painter().circle_stroke(
            rect.center(),
            10.5,
            egui::Stroke::new(2.0, ui.visuals().text_color()),
        );
    }
    response
}

const ACCENT_PRESETS: &[(&str, [u8; 3])] = &[
    ("Amber", [232, 181, 102]),
    ("Coral", [232, 132, 107]),
    ("Mint", [124, 201, 154]),
    ("Sky", [126, 177, 214]),
    ("Lilac", [180, 155, 224]),
    ("Rose", [229, 138, 177]),
];

fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(theme::muted(ui)));
}

fn permission_label(policy: PermissionPolicy) -> &'static str {
    match policy {
        PermissionPolicy::Ask => "Ask every time",
        PermissionPolicy::AutoAllowReads => "Auto-allow reads",
        PermissionPolicy::Yolo => "YOLO (allow all)",
    }
}
