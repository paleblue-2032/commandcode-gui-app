use egui::{ColorImage, Context, TextureHandle, TextureOptions};

pub struct Logos {
    wordmark_dark: TextureHandle,
    wordmark_light: TextureHandle,
    icon: TextureHandle,
}

impl Logos {
    pub fn load(ctx: &Context) -> Self {
        Self {
            wordmark_dark: load_texture(
                ctx,
                "commandcode-wordmark-white",
                include_bytes!("../assets/commandcode-wordmark-white.png"),
            ),
            wordmark_light: load_texture(
                ctx,
                "commandcode-wordmark-black",
                include_bytes!("../assets/commandcode-wordmark-black.png"),
            ),
            icon: load_texture(
                ctx,
                "commandcode-icon",
                include_bytes!("../assets/commandcode-icon.png"),
            ),
        }
    }

    pub fn show_wordmark(&self, ui: &mut egui::Ui, height: f32) -> egui::Response {
        let texture = if ui.visuals().dark_mode {
            &self.wordmark_dark
        } else {
            &self.wordmark_light
        };
        let size = texture.size_vec2();
        let width = height * size.x / size.y;
        ui.add(egui::Image::from_texture((
            texture.id(),
            egui::vec2(width, height),
        )))
    }

    pub fn show_icon(&self, ui: &mut egui::Ui, size: f32) -> egui::Response {
        ui.add(egui::Image::from_texture((
            self.icon.id(),
            egui::vec2(size, size),
        )))
    }
}

fn load_texture(ctx: &Context, name: &str, bytes: &[u8]) -> TextureHandle {
    let image = image::load_from_memory(bytes)
        .expect("bundled logo must be a valid image")
        .into_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.into_raw();
    ctx.load_texture(
        name,
        ColorImage::from_rgba_unmultiplied(size, &pixels),
        TextureOptions::LINEAR,
    )
}
