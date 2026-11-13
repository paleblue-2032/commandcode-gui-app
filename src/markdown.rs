use egui_commonmark::CommonMarkViewer;

pub fn show(
    ui: &mut egui::Ui,
    cache: &mut egui_commonmark::CommonMarkCache,
    id: egui::Id,
    text: &str,
) {
    CommonMarkViewer::new().show_with_id(id, ui, cache, text);
}
