use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Vec2};

pub const ACCENT: Color32 = Color32::from_rgb(232, 181, 102);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(118, 92, 52);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(47, 39, 24);
pub const SUCCESS: Color32 = Color32::from_rgb(124, 201, 154);
pub const WARNING: Color32 = Color32::from_rgb(230, 195, 106);
pub const DANGER: Color32 = Color32::from_rgb(230, 119, 119);
pub const INFO: Color32 = Color32::from_rgb(126, 177, 214);

pub fn apply(ctx: &egui::Context) {
    let base = (*ctx.global_style()).clone();

    let mut dark = base.clone();
    dark.visuals = egui::Visuals::dark();
    customize(&mut dark, true);

    let mut light = base;
    light.visuals = egui::Visuals::light();
    customize(&mut light, false);

    ctx.set_style_of(egui::Theme::Dark, dark);
    ctx.set_style_of(egui::Theme::Light, light);
}

fn customize(style: &mut egui::Style, dark: bool) {
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(10.0, 6.0);
    style.spacing.interact_size = Vec2::new(40.0, 30.0);
    style.spacing.window_margin = Margin::same(14);
    style.spacing.menu_margin = Margin::same(8);
    style.spacing.indent = 14.0;
    style.spacing.combo_width = 160.0;
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.floating = true;
    style.spacing.scroll.bar_inner_margin = 4.0;
    style.spacing.scroll.bar_outer_margin = 2.0;

    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(20.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(14.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(11.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(12.5, FontFamily::Monospace),
        ),
    ]
    .into_iter()
    .collect();

    let text = if dark {
        Color32::from_rgb(232, 229, 222)
    } else {
        Color32::from_rgb(31, 33, 38)
    };
    let weak = if dark {
        Color32::from_rgb(146, 151, 160)
    } else {
        Color32::from_rgb(104, 109, 118)
    };
    let border = border(dark);
    let window = if dark {
        Color32::from_rgb(24, 27, 33)
    } else {
        Color32::from_rgb(255, 255, 255)
    };
    let panel = if dark {
        Color32::from_rgb(16, 18, 22)
    } else {
        Color32::from_rgb(246, 245, 241)
    };
    let extreme = if dark {
        Color32::from_rgb(11, 13, 16)
    } else {
        Color32::from_rgb(238, 237, 232)
    };
    let faint = if dark {
        Color32::from_rgb(22, 25, 31)
    } else {
        Color32::from_rgb(240, 239, 234)
    };
    let inactive = if dark {
        Color32::from_rgb(28, 32, 39)
    } else {
        Color32::from_rgb(247, 246, 242)
    };
    let hovered = if dark {
        Color32::from_rgb(37, 42, 51)
    } else {
        Color32::from_rgb(237, 235, 229)
    };
    let accent_soft = if dark {
        ACCENT_SOFT
    } else {
        Color32::from_rgb(250, 240, 221)
    };

    let visuals = &mut style.visuals;
    visuals.dark_mode = dark;
    visuals.override_text_color = Some(text);
    visuals.weak_text_color = Some(weak);
    visuals.panel_fill = panel;
    visuals.window_fill = window;
    visuals.extreme_bg_color = extreme;
    visuals.text_edit_bg_color = Some(extreme);
    visuals.faint_bg_color = faint;
    visuals.code_bg_color = faint;
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = WARNING;
    visuals.error_fg_color = DANGER;
    visuals.selection.bg_fill = accent_soft;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT_DIM);
    visuals.window_corner_radius = CornerRadius::same(14);
    visuals.menu_corner_radius = CornerRadius::same(10);
    visuals.window_stroke = Stroke::new(1.0, border);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 10],
        blur: 28,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 130 } else { 30 }),
    };
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 110 } else { 24 }),
    };
    visuals.button_frame = true;
    visuals.collapsing_header_frame = false;
    visuals.indent_has_left_vline = false;
    visuals.striped = false;

    visuals.widgets.noninteractive.bg_fill = window;
    visuals.widgets.noninteractive.weak_bg_fill = window;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(8);

    visuals.widgets.inactive.bg_fill = inactive;
    visuals.widgets.inactive.weak_bg_fill = inactive;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(8);

    visuals.widgets.hovered.bg_fill = hovered;
    visuals.widgets.hovered.weak_bg_fill = hovered;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT_DIM);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(8);

    visuals.widgets.active.bg_fill = accent_soft;
    visuals.widgets.active.weak_bg_fill = accent_soft;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(
        1.0,
        if dark {
            ACCENT
        } else {
            Color32::from_rgb(101, 72, 21)
        },
    );
    visuals.widgets.active.corner_radius = CornerRadius::same(8);
}

pub fn border(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(42, 47, 56)
    } else {
        Color32::from_rgb(222, 220, 214)
    }
}

pub fn surface(ui: &egui::Ui) -> Color32 {
    ui.visuals().window_fill
}

pub fn muted(ui: &egui::Ui) -> Color32 {
    ui.visuals()
        .weak_text_color
        .unwrap_or(ui.visuals().text_color())
}

pub fn muted_color() -> Color32 {
    Color32::from_rgb(140, 146, 155)
}

pub fn card_frame(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::NONE
        .fill(surface(ui))
        .stroke(Stroke::new(1.0, border(ui.visuals().dark_mode)))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(12))
}

pub fn brand(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(22.0, 22.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(6), ACCENT);
    ui.painter()
        .circle_filled(rect.center(), 3.8, Color32::from_rgb(24, 20, 12));
}

pub fn accent_button(text: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(text)
        .fill(ACCENT)
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(8))
        .min_size(Vec2::new(0.0, 30.0))
}
