use tracing_subscriber::EnvFilter;

use commandcode_gui_app::{app, settings};

fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let settings = settings::Settings::load();
    let size = [
        settings.window_size[0].max(640.0),
        settings.window_size[1].max(520.0),
    ];
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Command Code GUI")
            .with_inner_size(size)
            .with_min_inner_size([420.0, 360.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Command Code GUI",
        options,
        Box::new(move |cc| {
            let app = app::App::new(cc, settings);
            Ok(Box::new(app))
        }),
    )
}
