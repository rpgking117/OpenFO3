//! The launcher's look: a dark theme with a burnt-orange accent, Windows'
//! own Segoe UI and icon fonts (read from the system at run time, never
//! shipped), the banner, the tab row, cards, switches and the Play button.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use eframe::egui::{
    self, text::LayoutJob, Align, Color32, CornerRadius, FontFamily, FontId, Margin, RichText,
    Sense, Stroke, TextFormat, Vec2,
};

/// Burnt orange.
pub const ACCENT: Color32 = Color32::from_rgb(0xCC, 0x55, 0x00);
/// A lighter orange, for text on dark.
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(0xFF, 0x8A, 0x3D);
pub const BG: Color32 = Color32::from_rgb(0x14, 0x15, 0x18);
pub const PANEL: Color32 = Color32::from_rgb(0x1C, 0x1E, 0x22);
pub const CARD: Color32 = Color32::from_rgb(0x22, 0x25, 0x2A);
pub const CARD_EDGE: Color32 = Color32::from_rgb(0x32, 0x36, 0x3D);
pub const FIELD: Color32 = Color32::from_rgb(0x17, 0x18, 0x1B);
pub const ROW_ALT: Color32 = Color32::from_rgb(0x1D, 0x1F, 0x23);
pub const TEXT: Color32 = Color32::from_rgb(0xE8, 0xE5, 0xE0);
pub const MUTED: Color32 = Color32::from_rgb(0x96, 0x99, 0xA0);
pub const GOOD: Color32 = Color32::from_rgb(0x6B, 0xC2, 0x6B);
pub const BAD: Color32 = Color32::from_rgb(0xE0, 0x6C, 0x5E);
pub const MOD_BLUE: Color32 = Color32::from_rgb(0x74, 0xAC, 0xE0);
const OFF: Color32 = Color32::from_rgb(0x2C, 0x2F, 0x35);

// Glyphs of Segoe Fluent Icons (Windows 11) / Segoe MDL2 Assets (Windows 10).
pub const FOLDER: char = '\u{E8B7}';
pub const FOLDER_OPEN: char = '\u{E838}';
pub const MONITOR: char = '\u{E7F4}';
pub const GEAR: char = '\u{E713}';
pub const IMPORT: char = '\u{E896}';
pub const INFO: char = '\u{E946}';
pub const PLAY: char = '\u{E768}';
pub const SEARCH: char = '\u{E721}';
pub const REFRESH: char = '\u{E72C}';
pub const UP: char = '\u{E70E}';
pub const DOWN: char = '\u{E70D}';
pub const CHECK: char = '\u{E73E}';
pub const WARNING: char = '\u{E7BA}';

static HAS_ICONS: AtomicBool = AtomicBool::new(false);

/// An icon: the Windows icon font's glyph, else a stand-in from egui's own
/// emoji font.
pub fn ic(c: char) -> String {
    if HAS_ICONS.load(Ordering::Relaxed) {
        return c.to_string();
    }
    match c {
        FOLDER => "📁",
        FOLDER_OPEN => "📂",
        MONITOR => "🖥",
        GEAR => "⚙",
        IMPORT => "📥",
        INFO => "ℹ",
        PLAY => "▶",
        SEARCH => "🔍",
        REFRESH => "🔄",
        UP => "▲",
        DOWN => "▼",
        CHECK => "✔",
        WARNING => "⚠",
        _ => "•",
    }
    .to_string()
}

fn icons() -> FontFamily {
    FontFamily::Name("icons".into())
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

/// Segoe UI, its semibold and the icon font from the Windows font folder,
/// when there (egui's own fonts stay behind them for anything missing).
pub fn fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let dir = std::env::var_os("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "C:\\Windows".into())
        .join("Fonts");
    let read = |name: &str| std::fs::read(dir.join(name)).ok();
    let mut body = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    if let Some(b) = read("segoeui.ttf") {
        fonts
            .font_data
            .insert("segoe".into(), Arc::new(egui::FontData::from_owned(b)));
        body.insert(0, "segoe".into());
    }
    let mut semi = body.clone();
    if let Some(b) = read("seguisb.ttf") {
        fonts
            .font_data
            .insert("segoe-semibold".into(), Arc::new(egui::FontData::from_owned(b)));
        semi.insert(0, "segoe-semibold".into());
    }
    let mut icon_list = body.clone();
    for name in ["SegoeIcons.ttf", "segmdl2.ttf"] {
        if let Some(b) = read(name) {
            fonts
                .font_data
                .insert("icons".into(), Arc::new(egui::FontData::from_owned(b)));
            icon_list.insert(0, "icons".into());
            HAS_ICONS.store(true, Ordering::Relaxed);
            break;
        }
    }
    fonts.families.insert(FontFamily::Proportional, body);
    fonts.families.insert(FontFamily::Name("semibold".into()), semi);
    fonts.families.insert(icons(), icon_list);
    ctx.set_fonts(fonts);
}

pub fn style(ctx: &egui::Context) {
    fonts(ctx);
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = PANEL;
    v.extreme_bg_color = FIELD;
    v.faint_bg_color = ROW_ALT;
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = ACCENT;
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.hyperlink_color = ACCENT_LIGHT;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, CARD_EDGE);
    v.widgets.inactive.bg_fill = OFF;
    v.widgets.inactive.weak_bg_fill = OFF;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, CARD_EDGE);
    v.widgets.hovered.bg_fill = Color32::from_rgb(0x38, 0x3B, 0x42);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x38, 0x3B, 0x42);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.weak_bg_fill = ACCENT;
    v.widgets.open.weak_bg_fill = OFF;
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = CornerRadius::same(5);
    }
    // Dark whatever Windows' own theme is (eframe would follow it).
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, v);
    ctx.style_mut_of(egui::Theme::Dark, |s| {
        s.spacing.item_spacing = Vec2::new(8.0, 8.0);
        s.spacing.button_padding = Vec2::new(12.0, 6.0);
        s.spacing.interact_size.y = 28.0;
        s.interaction.selectable_labels = false;
        use egui::TextStyle;
        s.text_styles.insert(TextStyle::Body, FontId::proportional(14.5));
        s.text_styles.insert(TextStyle::Button, FontId::proportional(14.5));
        s.text_styles.insert(TextStyle::Small, FontId::proportional(12.5));
        s.text_styles.insert(TextStyle::Heading, semibold(20.0));
        s.text_styles.insert(TextStyle::Monospace, FontId::monospace(13.0));
    });
}

/// Text with an icon in front.
pub fn icon_text(icon: char, text: &str, size: f32, color: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let format = |font_id| TextFormat {
        font_id,
        color,
        valign: Align::Center,
        ..Default::default()
    };
    job.append(&ic(icon), 0.0, format(FontId::new(size, icons())));
    if !text.is_empty() {
        job.append(text, size * 0.55, format(FontId::proportional(size)));
    }
    job
}

/// A button with an icon.
pub fn icon_button(icon: char, text: &str) -> egui::Button<'static> {
    egui::Button::new(icon_text(icon, text, 14.5, TEXT))
}

/// A titled section on a page.
pub fn card<R>(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let r = egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, CARD_EDGE))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(20, 16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(title.to_uppercase())
                    .font(semibold(12.5))
                    .extra_letter_spacing(1.2)
                    .color(ACCENT_LIGHT),
            );
            ui.add_space(8.0);
            add(ui)
        })
        .inner;
    ui.add_space(14.0);
    r
}

/// A note under a setting.
pub fn note(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(MUTED));
}

/// A setting: its name and note in a column on the left, the control on
/// the right.
pub fn setting(ui: &mut egui::Ui, label: &str, help: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(200.0, 0.0),
            egui::Layout::top_down(Align::Min),
            |ui| {
                ui.set_width(200.0);
                ui.label(RichText::new(label).font(semibold(14.5)));
                if !help.is_empty() {
                    note(ui, help);
                }
            },
        );
        ui.add_space(12.0);
        ui.vertical(add);
    });
    ui.add_space(12.0);
}

/// A switch (after egui's demo toggle): `on` flips when clicked.
pub fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = Vec2::new(42.0, 22.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool_responsive(response.id, *on);
        let mut fill = Color32::from_rgb(0x46, 0x49, 0x50).lerp_to_gamma(ACCENT, t);
        let mut knob = Color32::WHITE;
        if !ui.is_enabled() {
            fill = fill.gamma_multiply(0.55);
            knob = knob.gamma_multiply(0.7);
        }
        let r = rect.height() / 2.0;
        ui.painter()
            .rect_filled(rect, CornerRadius::same(r as u8), fill);
        let x = egui::lerp((rect.left() + r)..=(rect.right() - r), t);
        ui.painter()
            .circle_filled(egui::pos2(x, rect.center().y), r - 4.0, knob);
    }
    response
}

/// A row of choices, the chosen one lit. True when it changed.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    value: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let last = options.len().saturating_sub(1);
        for (i, (v, label)) in options.iter().enumerate() {
            let on = *value == *v;
            let (l, r) = (if i == 0 { 6 } else { 0 }, if i == last { 6 } else { 0 });
            let corner = CornerRadius {
                nw: l,
                sw: l,
                ne: r,
                se: r,
            };
            let text = RichText::new(*label).color(if on { Color32::WHITE } else { TEXT });
            let b = egui::Button::new(text)
                .fill(if on { ACCENT } else { OFF })
                .stroke(Stroke::new(1.0, if on { ACCENT } else { CARD_EDGE }))
                .corner_radius(corner)
                .min_size(Vec2::new(116.0, 32.0));
            if ui.add(b).clicked() && !on {
                *value = *v;
                changed = true;
            }
        }
    });
    changed
}

/// A path in a sunken field.
pub fn path_field(ui: &mut egui::Ui, path: Option<&Path>, empty: &str) {
    egui::Frame::new()
        .fill(FIELD)
        .stroke(Stroke::new(1.0, CARD_EDGE))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            match path {
                Some(p) => ui.add(
                    egui::Label::new(RichText::new(p.display().to_string()).monospace()).truncate(),
                ),
                None => ui.label(RichText::new(empty).color(MUTED)),
            };
        });
}

/// A small coloured tag (painted at its own size, so rows don't stretch it).
pub fn tag(ui: &mut egui::Ui, text: &str, color: Color32) {
    let mut job = LayoutJob::default();
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: semibold(10.5),
            color,
            extra_letter_spacing: 0.6,
            ..Default::default()
        },
    );
    let galley = ui.painter().layout_job(job);
    let (rect, _) = ui.allocate_exact_size(galley.size() + Vec2::new(14.0, 6.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(4), color.gamma_multiply(0.14));
    p.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, color.gamma_multiply(0.55)),
        egui::StrokeKind::Inside,
    );
    p.galley(rect.center() - galley.size() / 2.0, galley, color);
}

/// A status light.
pub fn dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), 7.0, color.gamma_multiply(0.25));
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

/// The banner across the top.
pub fn banner(ui: &mut egui::Ui, credit: &str, version: &str) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 104.0), Sense::hover());
    let p = ui.painter();
    // Dark, warming to burnt orange at the right.
    let mut mesh = egui::Mesh::default();
    let dark = Color32::from_rgb(0x17, 0x18, 0x1B);
    let warm = Color32::from_rgb(0x52, 0x24, 0x06);
    mesh.colored_vertex(rect.left_top(), dark);
    mesh.colored_vertex(rect.right_top(), warm);
    mesh.colored_vertex(rect.right_bottom(), warm);
    mesh.colored_vertex(rect.left_bottom(), dark);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(mesh);
    // Faint hazard stripes behind the credit.
    let stripe = Stroke::new(14.0, ACCENT.gamma_multiply(0.07));
    for k in 0..9 {
        let x = rect.right() - 30.0 - k as f32 * 34.0;
        p.line_segment(
            [egui::pos2(x - 40.0, rect.bottom()), egui::pos2(x, rect.top())],
            stripe,
        );
    }
    p.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(2.0, ACCENT),
    );
    let left = rect.left() + 28.0;
    p.text(
        egui::pos2(left - 2.0, rect.center().y - 12.0),
        egui::Align2::LEFT_CENTER,
        "OpenFO3",
        semibold(42.0),
        Color32::WHITE,
    );
    p.text(
        egui::pos2(left, rect.center().y + 24.0),
        egui::Align2::LEFT_CENTER,
        "An open-source engine for Fallout 3",
        FontId::proportional(15.0),
        Color32::from_rgb(0xD0, 0xC8, 0xBE),
    );
    p.text(
        egui::pos2(rect.right() - 28.0, rect.center().y - 9.0),
        egui::Align2::RIGHT_CENTER,
        credit,
        semibold(14.5),
        ACCENT_LIGHT,
    );
    p.text(
        egui::pos2(rect.right() - 28.0, rect.center().y + 14.0),
        egui::Align2::RIGHT_CENTER,
        format!("Version {version}"),
        FontId::proportional(12.5),
        Color32::from_rgb(0xB8, 0xAC, 0xA0),
    );
}

/// One tab of the row under the banner: an icon beside its name, the chosen
/// one underlined. True when clicked.
pub fn tab(ui: &mut egui::Ui, icon: char, label: &str, chosen: bool) -> bool {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        semibold(14.5),
        Color32::WHITE,
    );
    let size = Vec2::new(galley.size().x + 64.0, 50.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let p = ui.painter();
    if response.hovered() && !chosen {
        p.rect_filled(rect.shrink(4.0), CornerRadius::same(6), CARD);
    }
    if chosen {
        p.line_segment(
            [
                egui::pos2(rect.left() + 8.0, rect.bottom() - 1.5),
                egui::pos2(rect.right() - 8.0, rect.bottom() - 1.5),
            ],
            Stroke::new(3.0, ACCENT),
        );
    }
    let color = if chosen {
        Color32::WHITE
    } else if response.hovered() {
        TEXT
    } else {
        MUTED
    };
    let icon_color = if chosen { ACCENT_LIGHT } else { color };
    p.text(
        egui::pos2(rect.left() + 18.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        ic(icon),
        FontId::new(17.0, icons()),
        icon_color,
    );
    p.text(
        egui::pos2(rect.left() + 46.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        semibold(14.5),
        color,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
}

/// The big Play button.
pub fn play_button(ui: &mut egui::Ui, enabled: bool) -> bool {
    let job = icon_text(PLAY, "PLAY", 19.0, Color32::WHITE);
    let button = egui::Button::new(job)
        .fill(if enabled {
            ACCENT
        } else {
            Color32::from_rgb(0x4A, 0x36, 0x2A)
        })
        .stroke(Stroke::new(1.0, if enabled { ACCENT_LIGHT } else { CARD_EDGE }))
        .corner_radius(CornerRadius::same(6))
        .min_size(Vec2::new(176.0, 48.0));
    let r = ui.add_enabled(enabled, button);
    if r.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    r.clicked()
}

/// The window's icon (drawn by tools/release/make_icon.py).
pub fn app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/openfo3.png")).unwrap_or_default()
}
