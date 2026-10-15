use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, CursorIcon, Layout, Rect, RichText, Sense, Stroke, StrokeKind, Ui};

use super::{mouse_art, theme::*, HidraApp};
use crate::catalog::{action_label, ACTION_GROUPS, CONTROLS};
use crate::config::default_buttons;

pub fn show(app: &mut HidraApp, ui: &mut Ui) {
    let changed = CONTROLS.iter().filter(|c| app.cfg.buttons.get(c.id).map(String::as_str) != Some(c.default_action)).count();
    ui.horizontal(|ui| {
        ui.vertical(|ui| page_title(ui, "Buttons", "Pick a control on the mouse, then choose what it does."));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if button(ui, ButtonKind::Secondary, Some(Icon::Reset), "Restore defaults").clicked() {
                app.cfg.buttons = default_buttons();
            }
            let text = match changed {
                0 => "All controls at default".to_string(),
                1 => "1 control changed".to_string(),
                n => format!("{n} controls changed"),
            };
            ui.label(RichText::new(text).font(regular(13.0)).color(SUBTLE));
        });
    });
    ui.add_space(6.0);
    notice(
        ui,
        Tone::Info,
        "Mappings are saved on this computer. Writing them into the mouse's onboard memory comes in the next version, so for now the mouse keeps its own button layout.",
    );
    ui.add_space(8.0);

    let sel = app.sel_control.min(CONTROLS.len() - 1);
    let name = app.snap.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "G502 X".into());
    split(
        ui,
        0.47,
        app,
        |ui, app| {
            card(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    if let Some(i) = mouse_art::show(ui, sel) {
                        app.sel_control = i;
                    }
                    ui.add_space(6.0);
                    caption(ui, format!("{name} · top view"));
                });
            });
        },
        |ui, app| {
            card(ui, |ui| {
                section_title(ui, "Controls");
                ui.add_space(4.0);
                ui.spacing_mut().item_spacing.y = 6.0;
                for (i, c) in CONTROLS.iter().enumerate() {
                    let action = app.cfg.buttons.get(c.id).map(|a| action_label(a)).unwrap_or("");
                    if control_row(ui, c.tag, c.name, action, i == sel).clicked() {
                        app.sel_control = i;
                    }
                }
            });
        },
    );
    ui.add_space(GAP - 10.0);

    let control = &CONTROLS[sel];
    let current = app.cfg.buttons.get(control.id).cloned().unwrap_or_default();
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(control.name).font(semibold(18.0)).color(TEXT));
            ui.add_space(6.0);
            ui.label(RichText::new(format!("{} · now: {}", control.tag, action_label(&current))).font(mono(12.0)).color(SUBTLE));
        });
        ui.add_space(8.0);
        for group in ACTION_GROUPS {
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(vec2(104.0, 36.0), Layout::left_to_right(Align::Center), |ui| {
                    ui.set_width(104.0);
                    overline(ui, group.name);
                });
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                        for (id, label) in group.actions {
                            if chip(ui, label, current == *id).clicked() {
                                app.cfg.buttons.insert(control.id.to_string(), id.to_string());
                            }
                        }
                    });
                });
            });
            ui.add_space(4.0);
        }

        if current == "cmd" || current == "key" {
            ui.add_space(4.0);
            inset(ui, |ui| {
                let (title, hint, map) = if current == "cmd" {
                    ("Command", "flameshot gui", &mut app.cfg.commands)
                } else {
                    ("Shortcut", "Ctrl+Shift+T", &mut app.cfg.shortcuts)
                };
                ui.label(RichText::new(title).font(semibold(13.0)).color(TEXT));
                let value = map.entry(control.id.to_string()).or_default();
                ui.add(
                    egui::TextEdit::singleline(value)
                        .hint_text(hint)
                        .font(mono(14.0))
                        .desired_width(f32::INFINITY)
                        .margin(vec2(12.0, 10.0)),
                );
                caption(
                    ui,
                    if current == "cmd" {
                        "Runs as your user when the button is pressed."
                    } else {
                        "Write keys joined with +, for example Super+E."
                    },
                );
            });
        }
    });
}

fn control_row(ui: &mut Ui, tag: &str, name: &str, action: &str, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), Sense::click());
    let p = ui.painter();
    let (fill, border) = if selected {
        (accent_soft(), ACCENT)
    } else if resp.hovered() {
        (Color32::from_rgb(0x20, 0x24, 0x2B), Color32::from_rgb(0x3A, 0x40, 0x4A))
    } else {
        (RAISED, Color32::from_rgb(0x2A, 0x2F, 0x37))
    };
    p.rect(rect, 10, fill, Stroke::new(1.0, border), StrokeKind::Inside);
    let b = Rect::from_min_size(pos2(rect.left() + 12.0, rect.center().y - 11.0), vec2(32.0, 22.0));
    badge(p, b, tag, BADGE, TEXT_2, mono(11.0));
    p.text(pos2(b.right() + 12.0, rect.center().y), Align2::LEFT_CENTER, name, regular(14.0), TEXT);
    p.text(pos2(rect.right() - 14.0, rect.center().y), Align2::RIGHT_CENTER, action, regular(13.0), SUBTLE);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}
