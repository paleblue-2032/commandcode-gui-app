use egui::RichText;

use crate::diff;
use crate::model::{SessionView, TranscriptItem};
use crate::theme;
use crate::ui::chat;

pub fn draw(ui: &mut egui::Ui, session: &mut SessionView) {
    let Some(index) = session.selected_tool else {
        ui.label(RichText::new("Select a tool call to inspect it.").color(theme::muted(ui)));
        return;
    };
    let Some(TranscriptItem::Tool { call, .. }) = session.transcript.get(index) else {
        session.selected_tool = None;
        return;
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new("Tool details").strong().size(16.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Close").clicked() {
                session.selected_tool = None;
            }
        });
    });
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(8.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            theme::card_frame(ui).show(ui, |ui| {
                ui.label(RichText::new(call.title.as_str()).strong());
                chat::tool_body(ui, call);
            });

            if let Some(input) = &call.raw_input {
                ui.add_space(8.0);
                theme::card_frame(ui).show(ui, |ui| {
                    ui.label(RichText::new("Raw input").small().strong());
                    diff::show_json(ui, input);
                });
            }
            if let Some(output) = &call.raw_output {
                ui.add_space(8.0);
                theme::card_frame(ui).show(ui, |ui| {
                    ui.label(RichText::new("Raw output").small().strong());
                    diff::show_json(ui, output);
                });
            }
            ui.add_space(8.0);
        });
}
