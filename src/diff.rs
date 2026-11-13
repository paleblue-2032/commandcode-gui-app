use egui::{Color32, RichText};
use similar::{ChangeTag, TextDiff};

pub fn show_text_diff(ui: &mut egui::Ui, old_text: Option<&str>, new_text: &str) {
    let old_text = old_text.unwrap_or("");
    let diff = TextDiff::from_lines(old_text, new_text);
    let mut shown = 0usize;
    let limit = 2000usize;

    egui::Frame::group(ui.style()).show(ui, |ui| {
        egui::ScrollArea::vertical()
            .id_salt(("diff", new_text.len(), old_text.len()))
            .max_height(320.0)
            .show(ui, |ui| {
                ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                for change in diff.iter_all_changes() {
                    if shown >= limit {
                        ui.label(RichText::new("... diff truncated ...").italics());
                        break;
                    }
                    shown += 1;
                    let (sign, color) = match change.tag() {
                        ChangeTag::Delete => ("-", Color32::from_rgb(235, 110, 110)),
                        ChangeTag::Insert => ("+", Color32::from_rgb(110, 200, 130)),
                        ChangeTag::Equal => (" ", Color32::GRAY),
                    };
                    let line = change.value().trim_end_matches('\n');
                    ui.label(
                        RichText::new(format!("{sign}{line}"))
                            .monospace()
                            .color(color),
                    );
                }
            });
    });
}

pub fn show_json(ui: &mut egui::Ui, value: &serde_json::Value) {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    egui::Frame::group(ui.style()).show(ui, |ui| {
        egui::ScrollArea::vertical()
            .id_salt(("json", value.to_string().len()))
            .max_height(280.0)
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(RichText::new(text).monospace())
                        .selectable(true)
                        .wrap(),
                );
            });
    });
}
