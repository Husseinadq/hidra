use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, CursorIcon, Layout, Rect, RichText, Sense, Stroke, StrokeKind, Ui};

use super::{theme::*, HidraApp};
use crate::catalog::{action_label, control, ACTION_GROUPS, CONTROLS};
use crate::config::{AppProfile, Override};

const TILES: [Color32; 5] = [
    Color32::from_rgb(0xFF, 0x9B, 0x7A),
    Color32::from_rgb(0xFF, 0xB8, 0x70),
    Color32::from_rgb(0x8C, 0xC8, 0xFF),
    Color32::from_rgb(0xB7, 0xE0, 0x8A),
    Color32::from_rgb(0xC6, 0x9C, 0xFF),
];

pub fn show(app: &mut HidraApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| page_title(ui, "App profiles", "Different buttons for different apps."));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if button(ui, ButtonKind::Primary, Some(Icon::Plus), "New profile").clicked() {
                app.cfg.app_profiles.push(AppProfile { name: "New app".into(), wm_class: String::new(), enabled: true, overrides: Vec::new() });
                app.sel_app = app.cfg.app_profiles.len() - 1;
            }
        });
    });
    ui.add_space(6.0);
    notice(
        ui,
        Tone::Info,
        "Profiles are saved now. Switching them automatically needs a small GNOME Shell extension that reports the focused window; that's the next piece to build.",
    );
    ui.add_space(8.0);

    if app.cfg.app_profiles.is_empty() {
        card(ui, |ui| caption(ui, "No app profiles yet. Add one to change buttons for a single app."));
        return;
    }
    app.sel_app = app.sel_app.min(app.cfg.app_profiles.len() - 1);

    let mut delete = false;
    split(
        ui,
        0.4,
        app,
        |ui, app| {
            ui.spacing_mut().item_spacing.y = 8.0;
            for i in 0..app.cfg.app_profiles.len() {
                if profile_row(ui, &mut app.cfg.app_profiles[i], i, i == app.sel_app) {
                    app.sel_app = i;
                }
            }
        },
        |ui, app| delete = detail(app, ui),
    );
    if delete {
        app.cfg.app_profiles.remove(app.sel_app);
        app.sel_app = app.sel_app.saturating_sub(1);
    }
}

fn letter(name: &str) -> String {
    name.chars().find(|c| c.is_alphanumeric()).map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into())
}

fn summary(p: &AppProfile) -> String {
    let n = p.overrides.len();
    let count = if n == 1 { "1 override".to_string() } else { format!("{n} overrides") };
    if p.wm_class.is_empty() {
        format!("{count} · no window set")
    } else {
        format!("{count} · {}", p.wm_class)
    }
}

/// Returns true when the row (not its switch) was clicked.
fn profile_row(ui: &mut Ui, p: &mut AppProfile, i: usize, selected: bool) -> bool {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 64.0), Sense::click());
    let painter = ui.painter().clone();
    let (fill, border) = if selected {
        (accent_soft(), ACCENT)
    } else if resp.hovered() {
        (Color32::from_rgb(0x1B, 0x1F, 0x25), Color32::from_rgb(0x3A, 0x40, 0x4A))
    } else {
        (CARD, CARD_BORDER)
    };
    painter.rect(rect, 12, fill, Stroke::new(1.0, border), StrokeKind::Inside);
    let tile = Rect::from_min_size(rect.min + vec2(12.0, 12.0), vec2(40.0, 40.0));
    painter.rect_filled(tile, 10, TILES[i % TILES.len()]);
    painter.text(tile.center(), Align2::CENTER_CENTER, letter(&p.name), semibold(16.0), Color32::from_rgb(0x10, 0x13, 0x18));
    painter.text(pos2(tile.right() + 12.0, rect.top() + 23.0), Align2::LEFT_CENTER, &p.name, semibold(14.0), TEXT);
    painter.text(pos2(tile.right() + 12.0, rect.top() + 43.0), Align2::LEFT_CENTER, summary(p), regular(12.0), SUBTLE);

    let sw = Rect::from_min_size(pos2(rect.right() - 60.0, rect.center().y - 14.0), vec2(48.0, 28.0));
    let mut on = p.enabled;
    let toggled = toggle_at(ui, sw, &mut on).changed();
    if toggled {
        p.enabled = on;
    }
    resp.on_hover_cursor(CursorIcon::PointingHand).clicked() && !toggled
}

/// Returns true when the profile should be deleted.
fn detail(app: &mut HidraApp, ui: &mut Ui) -> bool {
    let i = app.sel_app;
    let mut delete = false;
    card(ui, |ui| {
        let p = &mut app.cfg.app_profiles[i];
        ui.horizontal(|ui| {
            let (tile, _) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
            ui.painter().rect_filled(tile, 12, TILES[i % TILES.len()]);
            ui.painter().text(tile.center(), Align2::CENTER_CENTER, letter(&p.name), semibold(20.0), Color32::from_rgb(0x10, 0x13, 0x18));
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new(&p.name).font(semibold(19.0)).color(TEXT));
                ui.label(RichText::new(summary(p)).font(regular(13.0)).color(SUBTLE));
            });
        });
        ui.add_space(8.0);

        field(ui, "Name", &mut p.name, "Firefox", false);
        field(ui, "Window class", &mut p.wm_class, "firefox", true);
        caption(ui, "The app's WM_CLASS or Wayland app id. Run `xprop WM_CLASS` or check Looking Glass to find it.");
        ui.add_space(8.0);

        overline(ui, "Overrides");
        let mut remove = None;
        egui::Frame::new().stroke(Stroke::new(1.0, Color32::from_rgb(0x2A, 0x2F, 0x37))).corner_radius(12).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            if p.overrides.is_empty() {
                caption(ui, "No overrides yet. This app uses the normal button layout.");
            }
            egui::Grid::new(("overrides", i)).num_columns(4).spacing(vec2(12.0, 10.0)).min_col_width(60.0).show(ui, |ui| {
                if !p.overrides.is_empty() {
                    for h in ["Control", "Default", "In this app", ""] {
                        ui.label(RichText::new(h).font(medium(12.0)).color(SUBTLE));
                    }
                    ui.end_row();
                }
                for (j, o) in p.overrides.iter_mut().enumerate() {
                    let ctl = control(&o.control);
                    egui::ComboBox::from_id_salt(("ctl", i, j)).width(170.0).selected_text(ctl.map(|c| c.name).unwrap_or("Pick a control")).show_ui(ui, |ui| {
                        for c in CONTROLS {
                            ui.selectable_value(&mut o.control, c.id.to_string(), c.name);
                        }
                    });
                    ui.label(RichText::new(ctl.map(|c| action_label(c.default_action)).unwrap_or("—")).font(regular(13.0)).color(SUBTLE));
                    egui::ComboBox::from_id_salt(("act", i, j)).width(170.0).selected_text(RichText::new(action_label(&o.action)).color(ACCENT)).show_ui(ui, |ui| {
                        for g in ACTION_GROUPS {
                            ui.label(RichText::new(g.name).font(semibold(11.0)).color(MUTED));
                            for (id, label) in g.actions {
                                ui.selectable_value(&mut o.action, id.to_string(), *label);
                            }
                        }
                    });
                    if icon_button(ui, Icon::Close, "Remove override").clicked() {
                        remove = Some(j);
                    }
                    ui.end_row();
                }
            });
        });
        if let Some(j) = remove {
            p.overrides.remove(j);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if button(ui, ButtonKind::Secondary, Some(Icon::Plus), "Add override").clicked() {
                let used: Vec<&str> = p.overrides.iter().map(|o| o.control.as_str()).collect();
                let next = CONTROLS.iter().find(|c| !used.contains(&c.id)).unwrap_or(&CONTROLS[0]);
                p.overrides.push(Override { control: next.id.to_string(), action: next.default_action.to_string() });
            }
            if button(ui, ButtonKind::Danger, Some(Icon::Trash), "Delete profile").clicked() {
                delete = true;
            }
        });
    });
    delete
}

fn field(ui: &mut Ui, label: &str, value: &mut String, hint: &str, monospace: bool) {
    ui.label(RichText::new(label).font(medium(13.0)).color(TEXT));
    let font = if monospace { mono(14.0) } else { regular(14.0) };
    ui.add(egui::TextEdit::singleline(value).hint_text(hint).font(font).desired_width(f32::INFINITY).margin(vec2(12.0, 9.0)));
    ui.add_space(4.0);
}
