//! Colours, fonts and the small set of custom widgets the design uses.

use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin, Painter, Pos2,
    Rect, Response, RichText, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2,
};

pub const BG: Color32 = Color32::from_rgb(0x0F, 0x11, 0x14);
pub const HEADER: Color32 = Color32::from_rgb(0x14, 0x17, 0x1B);
pub const SIDEBAR: Color32 = Color32::from_rgb(0x13, 0x16, 0x1A);
pub const CARD: Color32 = Color32::from_rgb(0x17, 0x1A, 0x1F);
pub const CARD_BORDER: Color32 = Color32::from_rgb(0x26, 0x2A, 0x31);
pub const RAISED: Color32 = Color32::from_rgb(0x1B, 0x1F, 0x25);
pub const CONTROL: Color32 = Color32::from_rgb(0x1F, 0x23, 0x2A);
pub const INSET: Color32 = Color32::from_rgb(0x13, 0x16, 0x1A);
pub const BORDER: Color32 = Color32::from_rgb(0x2E, 0x34, 0x3D);
pub const DIVIDER: Color32 = Color32::from_rgb(0x23, 0x27, 0x2E);
pub const BADGE: Color32 = Color32::from_rgb(0x26, 0x2B, 0x33);
pub const TEXT: Color32 = Color32::from_rgb(0xE8, 0xEA, 0xED);
pub const TEXT_2: Color32 = Color32::from_rgb(0xC4, 0xC9, 0xD1);
pub const SUBTLE: Color32 = Color32::from_rgb(0xA3, 0xA9, 0xB3);
pub const MUTED: Color32 = Color32::from_rgb(0x8A, 0x91, 0x9C);
pub const ACCENT: Color32 = Color32::from_rgb(0x5B, 0xB8, 0xFF);
pub const ACCENT_INK: Color32 = Color32::from_rgb(0x0B, 0x16, 0x22);
pub const GREEN: Color32 = Color32::from_rgb(0x7B, 0xD8, 0x8F);
pub const GREEN_BG: Color32 = Color32::from_rgb(0x2A, 0x3A, 0x2E);
pub const GREEN_TEXT: Color32 = Color32::from_rgb(0x9B, 0xE3, 0xAA);
pub const AMBER: Color32 = Color32::from_rgb(0xFF, 0xB8, 0x70);
pub const DANGER: Color32 = Color32::from_rgb(0xFF, 0x9B, 0x9B);
pub const DANGER_BORDER: Color32 = Color32::from_rgb(0x5A, 0x34, 0x34);
pub const KNOB_OFF: Color32 = Color32::from_rgb(0x3A, 0x40, 0x4A);

pub const GAP: f32 = 20.0;

pub fn accent_soft() -> Color32 {
    Color32::from_rgba_unmultiplied(0x5B, 0xB8, 0xFF, 0x24)
}

pub fn regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}
pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let fallbacks = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let add = |fonts: &mut FontDefinitions, name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(name.to_owned(), FontData::from_static(bytes).into());
    };
    add(&mut fonts, "plex", include_bytes!("../../assets/fonts/IBMPlexSans-Regular.ttf"));
    add(&mut fonts, "plex-medium", include_bytes!("../../assets/fonts/IBMPlexSans-Medium.ttf"));
    add(&mut fonts, "plex-semibold", include_bytes!("../../assets/fonts/IBMPlexSans-SemiBold.ttf"));
    add(&mut fonts, "plex-mono", include_bytes!("../../assets/fonts/IBMPlexMono-Medium.ttf"));
    let with = |first: &str| std::iter::once(first.to_owned()).chain(fallbacks.iter().cloned()).collect::<Vec<_>>();
    fonts.families.insert(FontFamily::Proportional, with("plex"));
    fonts.families.insert(FontFamily::Name("medium".into()), with("plex-medium"));
    fonts.families.insert(FontFamily::Name("semibold".into()), with("plex-semibold"));
    fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "plex-mono".to_owned());
    ctx.set_fonts(fonts);

    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, regular(12.0)),
            (TextStyle::Body, regular(14.0)),
            (TextStyle::Button, medium(13.0)),
            (TextStyle::Heading, semibold(20.0)),
            (TextStyle::Monospace, mono(13.0)),
        ]
        .into();
        style.spacing.item_spacing = vec2(8.0, 8.0);
        style.spacing.button_padding = vec2(14.0, 8.0);
        style.spacing.interact_size.y = 36.0;
        style.spacing.slider_width = 320.0;
        style.spacing.scroll.bar_width = 8.0;
        let v = &mut style.visuals;
        *v = egui::Visuals::dark();
        v.panel_fill = BG;
        v.window_fill = RAISED;
        v.extreme_bg_color = BG;
        v.faint_bg_color = RAISED;
        v.override_text_color = None;
        v.hyperlink_color = Color32::from_rgb(0x9F, 0xD3, 0xFF);
        v.selection.bg_fill = Color32::from_rgba_unmultiplied(0x5B, 0xB8, 0xFF, 0x60);
        v.selection.stroke = Stroke::new(1.0, ACCENT);
        v.slider_trailing_fill = true;
        v.window_corner_radius = CornerRadius::same(12);
        v.window_stroke = Stroke::new(1.0, BORDER);
        v.menu_corner_radius = CornerRadius::same(10);
        let w = &mut v.widgets;
        w.noninteractive.bg_stroke = Stroke::new(1.0, DIVIDER);
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_2);
        for (state, fill, stroke) in [
            (&mut w.inactive, CONTROL, BORDER),
            (&mut w.hovered, Color32::from_rgb(0x26, 0x2B, 0x33), Color32::from_rgb(0x44, 0x4B, 0x56)),
            (&mut w.active, Color32::from_rgb(0x2C, 0x32, 0x3B), ACCENT),
            (&mut w.open, CONTROL, BORDER),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = Stroke::new(1.0, stroke);
            state.corner_radius = CornerRadius::same(8);
            state.fg_stroke = Stroke::new(1.5, TEXT);
            state.expansion = 0.0;
        }
    });
}

// ---------------------------------------------------------------- layout

pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, CARD_BORDER))
        .corner_radius(16)
        .inner_margin(Margin::symmetric(22, 20))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

pub fn inset<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(INSET)
        .stroke(Stroke::new(1.0, Color32::from_rgb(0x2A, 0x2F, 0x37)))
        .corner_radius(12)
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// Two columns side by side, or stacked when the window is narrow.
pub fn split<T>(ui: &mut Ui, ratio: f32, state: &mut T, left: impl FnOnce(&mut Ui, &mut T), right: impl FnOnce(&mut Ui, &mut T)) {
    let w = ui.available_width();
    if w < 800.0 {
        left(ui, state);
        ui.add_space(GAP - 8.0);
        right(ui, state);
        return;
    }
    let lw = ((w - GAP) * ratio).floor();
    let rw = w - GAP - lw;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        ui.vertical(|ui| {
            ui.set_width(lw);
            left(ui, state)
        });
        ui.vertical(|ui| {
            ui.set_width(rw);
            right(ui, state)
        });
    });
}

pub fn page_title(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).font(semibold(26.0)).color(TEXT));
    ui.add_space(-2.0);
    ui.label(RichText::new(subtitle).font(regular(14.0)).color(SUBTLE));
}

pub fn section_title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).font(semibold(17.0)).color(TEXT));
}

pub fn overline(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text.to_uppercase()).font(semibold(11.0)).color(MUTED).extra_letter_spacing(1.0));
}

pub fn caption(ui: &mut Ui, text: impl Into<String>) {
    ui.label(RichText::new(text.into()).font(regular(12.5)).color(SUBTLE));
}

#[derive(Clone, Copy)]
pub enum Tone {
    Info,
    Warn,
}

pub fn notice(ui: &mut Ui, tone: Tone, text: &str) {
    let (stroke, icon_color) = match tone {
        Tone::Info => (Color32::from_rgb(0x2A, 0x3A, 0x4C), ACCENT),
        Tone::Warn => (Color32::from_rgb(0x4C, 0x3C, 0x2A), AMBER),
    };
    Frame::new().fill(CARD).stroke(Stroke::new(1.0, stroke)).corner_radius(12).inner_margin(Margin::symmetric(14, 12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_top(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
            icon(ui.painter(), r.center(), 16.0, Icon::Info, icon_color);
            wrapped(ui, RichText::new(text).font(regular(13.0)).color(TEXT_2));
        });
    });
}

/// A label that wraps to the remaining width, even inside a horizontal row
/// (where egui would otherwise lay it out on one long line).
pub fn wrapped(ui: &mut Ui, text: RichText) {
    let w = ui.available_width().max(80.0);
    ui.allocate_ui_with_layout(vec2(w, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_max_width(w);
        ui.add(egui::Label::new(text).wrap());
    });
}

// ---------------------------------------------------------------- widgets

fn text_galley(ui: &Ui, text: &str, font: FontId, color: Color32) -> std::sync::Arc<egui::Galley> {
    ui.painter().layout_no_wrap(text.to_owned(), font, color)
}

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Danger,
}

pub fn button(ui: &mut Ui, kind: ButtonKind, icon_kind: Option<Icon>, label: &str) -> Response {
    let enabled = ui.is_enabled();
    let (fill, border, fg) = match kind {
        ButtonKind::Primary => (ACCENT, ACCENT, ACCENT_INK),
        ButtonKind::Secondary => (RAISED, BORDER, TEXT),
        ButtonKind::Danger => (Color32::TRANSPARENT, DANGER_BORDER, DANGER),
    };
    let font = if kind == ButtonKind::Primary { semibold(13.0) } else { medium(13.0) };
    let g = text_galley(ui, label, font, fg);
    let icon_w = if icon_kind.is_some() { 22.0 } else { 0.0 };
    let size = vec2(g.size().x + 28.0 + icon_w, 38.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered() && enabled;
        let fill = if hovered {
            match kind {
                ButtonKind::Primary => Color32::from_rgb(0x7A, 0xC6, 0xFF),
                ButtonKind::Secondary => Color32::from_rgb(0x24, 0x29, 0x30),
                ButtonKind::Danger => Color32::from_rgba_unmultiplied(0xFF, 0x9B, 0x9B, 0x12),
            }
        } else {
            fill
        };
        let alpha = if enabled { 1.0 } else { 0.4 };
        let p = ui.painter();
        p.rect(rect, 10, fill.gamma_multiply(alpha), Stroke::new(1.0, border.gamma_multiply(alpha)), StrokeKind::Inside);
        let mut x = rect.left() + 14.0;
        if let Some(ic) = icon_kind {
            icon(p, pos2(x + 8.0, rect.center().y), 15.0, ic, fg.gamma_multiply(alpha));
            x += icon_w;
        }
        p.galley(pos2(x, rect.center().y - g.size().y / 2.0), g, fg.gamma_multiply(alpha));
    }
    if enabled {
        resp.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// A rounded chip used for picking one option from many.
pub fn chip(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let fg = if selected { ACCENT_INK } else { Color32::from_rgb(0xD6, 0xDA, 0xE0) };
    let g = text_galley(ui, label, medium(13.0), fg);
    let check = if selected { 20.0 } else { 0.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(g.size().x + 28.0 + check, 36.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = resp.hovered();
        let (fill, border) = if selected {
            (ACCENT, ACCENT)
        } else if hovered {
            (Color32::from_rgb(0x26, 0x2B, 0x33), Color32::from_rgb(0x44, 0x4B, 0x56))
        } else {
            (CONTROL, Color32::from_rgb(0x32, 0x38, 0x43))
        };
        let p = ui.painter();
        p.rect(rect, 18, fill, Stroke::new(1.0, border), StrokeKind::Inside);
        let mut x = rect.left() + 14.0;
        if selected {
            icon(p, pos2(x + 7.0, rect.center().y), 14.0, Icon::Check, fg);
            x += check;
        }
        p.galley(pos2(x, rect.center().y - g.size().y / 2.0), g, fg);
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// An on/off switch. Returns a response that is `changed()` when flipped.
pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
    let (rect, _) = ui.allocate_exact_size(vec2(48.0, 28.0), Sense::hover());
    toggle_at(ui, rect, on)
}

/// A switch drawn at a fixed rect without touching the layout cursor.
pub fn toggle_at(ui: &mut Ui, rect: Rect, on: &mut bool) -> Response {
    let mut resp = ui.interact(rect, ui.id().with(("toggle", rect.min.x as i32, rect.min.y as i32)), Sense::click());
    if resp.clicked() && ui.is_enabled() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    if ui.is_rect_visible(rect) {
        let alpha = if ui.is_enabled() { 1.0 } else { 0.4 };
        let track = lerp_color(KNOB_OFF, ACCENT, t).gamma_multiply(alpha);
        let p = ui.painter();
        p.rect_filled(rect, 14, track);
        let x = egui::lerp((rect.left() + 14.0)..=(rect.right() - 14.0), t);
        p.circle_filled(pos2(x, rect.center().y), 11.0, Color32::from_rgb(0xF4, 0xF5, 0xF7).gamma_multiply(alpha));
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

/// A row of equal-width segments; returns the index clicked, if any.
pub fn segmented(ui: &mut Ui, labels: &[String], selected: Option<usize>, font: FontId) -> Option<usize> {
    let w = ui.available_width();
    let h = 46.0;
    let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
    let p = ui.painter().clone();
    p.rect(rect, 12, BG, Stroke::new(1.0, Color32::from_rgb(0x2A, 0x2F, 0x37)), StrokeKind::Inside);
    let inner = rect.shrink(4.0);
    let n = labels.len().max(1) as f32;
    let seg_w = (inner.width() - 4.0 * (n - 1.0)) / n;
    let mut clicked = None;
    for (i, label) in labels.iter().enumerate() {
        let r = Rect::from_min_size(pos2(inner.left() + i as f32 * (seg_w + 4.0), inner.top()), vec2(seg_w, inner.height()));
        let resp = ui.interact(r, ui.id().with(("seg", label, i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let on = selected == Some(i);
        let enabled = ui.is_enabled();
        if on {
            p.rect_filled(r, 9, ACCENT.gamma_multiply(if enabled { 1.0 } else { 0.4 }));
        } else if resp.hovered() && enabled {
            p.rect_filled(r, 9, Color32::from_rgb(0x1F, 0x23, 0x2A));
        }
        let color = if on { ACCENT_INK } else { TEXT_2 };
        p.text(r.center(), Align2::CENTER_CENTER, label, font.clone(), color.gamma_multiply(if enabled { 1.0 } else { 0.5 }));
        if resp.clicked() && enabled {
            clicked = Some(i);
        }
    }
    clicked
}

pub fn badge(p: &Painter, rect: Rect, text: &str, fill: Color32, color: Color32, font: FontId) {
    p.rect_filled(rect, 5, fill);
    p.text(rect.center(), Align2::CENTER_CENTER, text, font, color);
}

pub fn status_dot(p: &Painter, center: Pos2, color: Color32) {
    p.circle_filled(center, 3.5, color);
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

pub fn fmt_thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

// ---------------------------------------------------------------- icons

#[derive(Clone, Copy)]
pub enum Icon {
    Logo,
    Mouse,
    Gauge,
    Grid,
    Chip,
    Plus,
    Reset,
    Check,
    Info,
    Minus,
    Square,
    Close,
    Refresh,
    Copy,
    Trash,
}

/// Stroke icons drawn on a 24-unit grid, centred on `c`.
pub fn icon(p: &Painter, c: Pos2, size: f32, kind: Icon, color: Color32) {
    let s = size / 24.0;
    let pt = |x: f32, y: f32| pos2(c.x + (x - 12.0) * s, c.y + (y - 12.0) * s);
    let st = Stroke::new((2.0 * s).max(1.4), color);
    let line = |pts: &[(f32, f32)]| {
        p.line(pts.iter().map(|&(x, y)| pt(x, y)).collect(), st);
    };
    let rrect = |x: f32, y: f32, w: f32, h: f32, r: f32| {
        p.rect_stroke(Rect::from_min_size(pt(x, y), vec2(w * s, h * s)), CornerRadius::same((r * s) as u8), st, StrokeKind::Middle);
    };
    match kind {
        Icon::Logo => {
            line(&[(12.0, 2.0), (19.0, 6.0), (19.0, 18.0), (12.0, 22.0), (5.0, 18.0), (5.0, 6.0), (12.0, 2.0)]);
            line(&[(12.0, 7.0), (12.0, 12.0)]);
        }
        Icon::Mouse => {
            rrect(6.0, 3.0, 12.0, 18.0, 6.0);
            line(&[(12.0, 7.0), (12.0, 11.0)]);
        }
        Icon::Gauge => {
            p.circle_stroke(pt(12.0, 13.0), 8.0 * s, st);
            line(&[(12.0, 13.0), (16.0, 9.0)]);
            line(&[(12.0, 2.0), (12.0, 5.0)]);
        }
        Icon::Grid => {
            for (x, y) in [(3.0, 3.0), (14.0, 3.0), (3.0, 14.0), (14.0, 14.0)] {
                rrect(x, y, 7.0, 7.0, 1.5);
            }
        }
        Icon::Chip => {
            rrect(4.0, 4.0, 16.0, 16.0, 2.0);
            rrect(9.0, 9.0, 6.0, 6.0, 0.0);
            for v in [9.0, 15.0] {
                line(&[(v, 1.0), (v, 4.0)]);
                line(&[(v, 20.0), (v, 23.0)]);
                line(&[(1.0, v), (4.0, v)]);
                line(&[(20.0, v), (23.0, v)]);
            }
        }
        Icon::Plus => {
            line(&[(12.0, 5.0), (12.0, 19.0)]);
            line(&[(5.0, 12.0), (19.0, 12.0)]);
        }
        Icon::Minus => line(&[(6.0, 12.0), (18.0, 12.0)]),
        Icon::Square => rrect(5.0, 5.0, 14.0, 14.0, 2.0),
        Icon::Close => {
            line(&[(6.0, 6.0), (18.0, 18.0)]);
            line(&[(18.0, 6.0), (6.0, 18.0)]);
        }
        Icon::Reset | Icon::Refresh => {
            let pts: Vec<Pos2> = (0..=20)
                .map(|i| {
                    let a = std::f32::consts::PI * (1.1 + 1.55 * i as f32 / 20.0);
                    pt(12.0 + 8.0 * a.cos(), 12.0 + 8.0 * a.sin())
                })
                .collect();
            let start = pts[0];
            p.line(pts, st);
            p.line(vec![start, pos2(start.x, start.y - 5.0 * s)], st);
            p.line(vec![start, pos2(start.x + 5.0 * s, start.y)], st);
        }
        Icon::Check => line(&[(5.0, 12.0), (10.0, 17.0), (20.0, 7.0)]),
        Icon::Info => {
            p.circle_stroke(pt(12.0, 12.0), 9.0 * s, st);
            line(&[(12.0, 11.0), (12.0, 16.0)]);
            p.circle_filled(pt(12.0, 7.8), 1.2 * s.max(0.8), color);
        }
        Icon::Copy => {
            rrect(8.0, 8.0, 12.0, 12.0, 2.0);
            line(&[(4.0, 15.0), (4.0, 6.0), (6.0, 4.0), (15.0, 4.0)]);
        }
        Icon::Trash => {
            line(&[(4.0, 7.0), (20.0, 7.0)]);
            line(&[(9.0, 7.0), (9.0, 4.0), (15.0, 4.0), (15.0, 7.0)]);
            line(&[(6.0, 7.0), (7.0, 20.0), (17.0, 20.0), (18.0, 7.0)]);
        }
    }
}

/// Allocates a square and draws an icon in it; handy inside layouts.
pub fn icon_widget(ui: &mut Ui, kind: Icon, size: f32, color: Color32) -> Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    icon(ui.painter(), r.center(), size, kind, color);
    resp
}

/// A small round icon button (window controls, remove buttons).
pub fn icon_button(ui: &mut Ui, kind: Icon, tooltip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::click());
    let fill = if resp.hovered() { Color32::from_rgb(0x2C, 0x32, 0x3B) } else { Color32::from_rgb(0x22, 0x26, 0x2D) };
    ui.painter().circle_filled(rect.center(), 16.0, fill);
    icon(ui.painter(), rect.center(), 14.0, kind, TEXT_2);
    resp.on_hover_text(tooltip).on_hover_cursor(CursorIcon::PointingHand)
}
