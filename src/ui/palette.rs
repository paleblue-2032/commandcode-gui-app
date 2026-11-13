use agent_client_protocol::v1::schema::AvailableCommand;
use egui::{CornerRadius, Margin, RichText, Stroke};

use crate::theme;

pub enum PaletteAction {
    Insert(String),
    Close,
}

pub fn draw(
    ctx: &egui::Context,
    filter: &mut String,
    commands: &[AvailableCommand],
) -> Option<PaletteAction> {
    let mut action = None;
    let style = ctx.style_of(ctx.theme());
    let frame = egui::Frame::NONE
        .fill(style.visuals.window_fill)
        .stroke(Stroke::new(1.0, theme::border(style.visuals.dark_mode)))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(18));

    let response = egui::Modal::new(egui::Id::unique("command_palette"))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_min_width((ctx.content_rect().width() - 72.0).clamp(300.0, 540.0));
            ui.label(RichText::new("Commands").strong().size(18.0));
            ui.label(
                RichText::new("Insert a slash command into the composer.")
                    .small()
                    .color(theme::muted(ui)),
            );
            ui.add_space(8.0);

            egui::Frame::NONE
                .fill(ui.visuals().extreme_bg_color)
                .corner_radius(8)
                .inner_margin(Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(filter)
                            .frame(egui::Frame::NONE)
                            .hint_text("Filter commands")
                            .desired_width(f32::INFINITY),
                    );
                });
            ui.add_space(8.0);

            let filter_lower = filter.to_lowercase();
            egui::ScrollArea::vertical()
                .max_height(380.0)
                .show(ui, |ui| {
                    let filtered: Vec<&AvailableCommand> = commands
                        .iter()
                        .filter(|command| {
                            filter_lower.is_empty()
                                || command.name.to_lowercase().contains(&filter_lower)
                                || command.description.to_lowercase().contains(&filter_lower)
                        })
                        .collect();

                    if filtered.is_empty() {
                        ui.label(RichText::new("No commands available").color(theme::muted(ui)));
                        return;
                    }

                    for command in filtered {
                        let button = egui::Button::new(
                            RichText::new(format!("/{}", command.name))
                                .monospace()
                                .color(ui.visuals().strong_text_color()),
                        )
                        .fill(ui.visuals().faint_bg_color)
                        .stroke(Stroke::new(1.0, theme::border(ui.visuals().dark_mode)))
                        .corner_radius(CornerRadius::same(8))
                        .min_size(egui::vec2(ui.available_width(), 32.0));
                        if ui.add(button).on_hover_text(&command.description).clicked() {
                            action = Some(PaletteAction::Insert(format!("/{} ", command.name)));
                        }
                        ui.label(
                            RichText::new(&command.description)
                                .small()
                                .color(theme::muted(ui)),
                        );
                        ui.add_space(6.0);
                    }
                });
        });

    if action.is_none() && response.should_close() {
        action = Some(PaletteAction::Close);
    }
    action
}
