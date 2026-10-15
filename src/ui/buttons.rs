use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, CursorIcon, Layout, Rect, RichText, Sense, Stroke, StrokeKind, Ui};

use super::{mouse_art, theme::*, HidraApp};
use crate::catalog::{action_label, needs_software, Control, ACTION_GROUPS, CONTROLS};
use crate::config::default_buttons;
use crate::device::OnboardMode;
use crate::profile;
use crate::worker::Command;

/// Whether changes can go straight into the mouse right now.
fn mouse_ready(app: &HidraApp) -> bool {
    app.connected() && app.snap.as_ref().is_some_and(|s| s.buttons.is_some())
}

fn assign(app: &mut HidraApp, c: &Control, action: &str) {
    if needs_software(action) {
        app.toast("Running commands needs Hidra's background engine, which isn't built yet.", true);
        return;
    }
    app.cfg.buttons.insert(c.id.to_string(), action.to_string());
    app.shortcut_error = None;
    if action == "key" {
        // Written once a valid shortcut is typed.
        let text = app.cfg.shortcuts.get(c.id).cloned().unwrap_or_default();
        if !text.trim().is_empty() {
            write_one(app, c, "key", Some(&text));
        }
        return;
    }
    write_one(app, c, action, None);
}

fn write_one(app: &mut HidraApp, c: &Control, action: &str, shortcut: Option<&str>) {
    if !mouse_ready(app) {
        return;
    }
    match profile::binding_for(action, shortcut) {
        Ok(raw) => {
            let label = if action == "key" { shortcut.unwrap_or("").to_string() } else { action_label(action).to_string() };
            app.send(Command::WriteButtons(vec![(c.slot, raw)], format!("{} → {} saved to the mouse", c.tag, label)));
        }
        Err(e) => app.shortcut_error = Some(e),
    }
}

pub fn show(app: &mut HidraApp, ui: &mut Ui) {
    let changed = CONTROLS.iter().filter(|c| app.cfg.buttons.get(c.id).map(String::as_str) != Some(c.default_action)).count();
    ui.horizontal(|ui| {
        ui.vertical(|ui| page_title(ui, "Buttons", "Pick a control on the mouse, then choose what it does."));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if button(ui, ButtonKind::Secondary, Some(Icon::Reset), "Restore defaults").clicked() {
                app.cfg.buttons = default_buttons();
                if mouse_ready(app) {
                    let raws = CONTROLS.iter().filter_map(|c| profile::binding_for(c.default_action, None).ok().map(|r| (c.slot, r))).collect();
                    app.send(Command::WriteButtons(raws, "Factory button layout restored".into()));
                }
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

    let snap = app.snap.as_ref();
    let host_mode = snap.and_then(|s| s.onboard.as_ref()).is_some_and(|o| o.mode == OnboardMode::Host);
    let slot = snap.and_then(|s| s.buttons.as_ref()).map(|b| b.sector);
    match (mouse_ready(app), host_mode, slot) {
        (true, false, Some(n)) => notice(
            ui,
            Tone::Info,
            &format!("Changes go straight into onboard profile {n}, so they work in every app, even when Hidra is closed. The original profile is backed up in ~/.config/hidra/backup."),
        ),
        (true, true, _) => notice(ui, Tone::Warn, "Onboard memory is off, so the mouse isn't using its saved buttons. Turn it on in Device & memory to use these."),
        _ => notice(ui, Tone::Warn, "Connect or wake the mouse to change its buttons. Choices made now are only saved in Hidra."),
    }
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
                        app.shortcut_error = None;
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
                    let action = app.cfg.buttons.get(c.id).map(String::as_str).unwrap_or("");
                    let label = if action == "key" {
                        app.cfg.shortcuts.get(c.id).cloned().filter(|s| !s.is_empty()).unwrap_or_else(|| "Keyboard shortcut".into())
                    } else {
                        action_label(action).to_string()
                    };
                    if control_row(ui, c.tag, c.name, &label, i == sel).clicked() {
                        app.sel_control = i;
                        app.shortcut_error = None;
                    }
                }
            });
        },
    );
    ui.add_space(GAP - 10.0);

    let control = &CONTROLS[sel];
    let current = app.cfg.buttons.get(control.id).cloned().unwrap_or_default();
    let mut picked = None;
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
                            let soft = needs_software(id);
                            let resp = ui.add_enabled_ui(!soft, |ui| chip(ui, label, current == *id)).inner;
                            if soft {
                                resp.on_disabled_hover_text("Needs Hidra's background engine — coming soon");
                            } else if resp.clicked() && current != *id {
                                picked = Some(*id);
                            }
                        }
                    });
                });
            });
            ui.add_space(4.0);
        }

        if current == "key" {
            ui.add_space(4.0);
            inset(ui, |ui| {
                ui.label(RichText::new("Shortcut").font(semibold(13.0)).color(TEXT));
                let value = app.cfg.shortcuts.entry(control.id.to_string()).or_default();
                let resp = ui.add(
                    egui::TextEdit::singleline(value)
                        .hint_text("Ctrl+Shift+T")
                        .font(mono(14.0))
                        .desired_width(f32::INFINITY)
                        .margin(vec2(12.0, 10.0)),
                );
                let text = value.clone();
                if resp.lost_focus() {
                    match profile::parse_shortcut(&text) {
                        Ok((m, k)) => {
                            let tidy = profile::format_shortcut(m, k);
                            app.cfg.shortcuts.insert(control.id.to_string(), tidy.clone());
                            app.shortcut_error = None;
                            write_one(app, control, "key", Some(&tidy));
                        }
                        Err(e) => app.shortcut_error = Some(e),
                    }
                }
                match &app.shortcut_error {
                    Some(e) => ui.label(RichText::new(e).font(regular(12.5)).color(DANGER)),
                    None => ui.label(
                        RichText::new("One key plus Ctrl, Shift, Alt or Super, for example Super+E. Press Enter to save it to the mouse.")
                            .font(regular(12.5))
                            .color(SUBTLE),
                    ),
                };
            });
        }
    });
    if let Some(id) = picked {
        assign(app, control, id);
    }
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
