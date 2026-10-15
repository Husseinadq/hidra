use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, CursorIcon, Layout, Rect, RichText, Sense, Stroke, StrokeKind, Ui};

use super::{theme::*, HidraApp};
use crate::desktop::{Accel, Desktop};
use crate::device::OnboardMode;
use crate::worker::Command;

pub fn show(app: &mut HidraApp, ui: &mut Ui) {
    page_title(ui, "Pointer & scroll", "Sensitivity and report rate go to the mouse. Speed and scrolling are desktop settings.");
    ui.add_space(6.0);
    if !app.connected() {
        notice(ui, Tone::Warn, "Connect or wake the mouse to change its sensitivity and report rate.");
        ui.add_space(8.0);
    }
    sensitivity(app, ui);
    ui.add_space(GAP - 10.0);
    split(ui, 0.5, app, |ui, app| pointer_card(app, ui), |ui, app| scroll_card(app, ui));
}

fn sensitivity(app: &mut HidraApp, ui: &mut Ui) {
    let caps = app.snap.as_ref().and_then(|s| s.dpi.clone());
    let live = caps.as_ref().map(|c| c.current);
    let (min, max, step) = caps.as_ref().map(|c| (c.min, c.max, c.step)).unwrap_or((100, 25_600, 50));
    let connected = app.connected();

    card(ui, |ui| {
        ui.horizontal(|ui| {
            section_title(ui, "Sensitivity stages");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let text = match live {
                    Some(v) => format!("On the mouse now: {} DPI", fmt_thousands(v as u32)),
                    None => "Mouse not connected".into(),
                };
                ui.label(RichText::new(text).font(regular(13.0)).color(SUBTLE));
            });
        });
        ui.add_space(6.0);

        // Stage tiles, wrapping on narrow windows.
        let w = ui.available_width();
        let per_row = (((w + 10.0) / 140.0).floor() as usize).clamp(1, 5);
        let tile_w = (w - 10.0 * (per_row as f32 - 1.0)) / per_row as f32;
        let mut picked = None;
        for row in (0..5).collect::<Vec<_>>().chunks(per_row) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                for &i in row {
                    let v = app.cfg.dpi_stages[i];
                    let resp = stage_tile(ui, tile_w, i, v, i == app.edit_stage, i == app.cfg.default_stage, live == Some(v));
                    if resp.clicked() {
                        picked = Some(i);
                    }
                }
            });
        }
        if let Some(i) = picked {
            app.edit_stage = i;
            app.dpi_draft = None;
            if connected {
                app.send(Command::SetDpi(app.cfg.dpi_stages[i]));
            }
        }

        ui.add_space(6.0);
        let i = app.edit_stage;
        inset(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(170.0);
                    ui.label(RichText::new(format!("Stage {}", i + 1)).font(regular(12.0)).color(SUBTLE));
                    let shown = app.dpi_draft.unwrap_or(app.cfg.dpi_stages[i]);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(fmt_thousands(shown as u32)).font(mono(32.0)).color(ACCENT));
                        ui.label(RichText::new("DPI").font(regular(14.0)).color(SUBTLE));
                    });
                });
                ui.vertical(|ui| {
                    let right = 190.0;
                    let slider_w = (ui.available_width() - right).max(160.0);
                    ui.label(RichText::new("DPI for this stage").font(medium(13.0)).color(TEXT));
                    let mut v = app.dpi_draft.unwrap_or(app.cfg.dpi_stages[i]);
                    ui.spacing_mut().slider_width = slider_w;
                    let resp = ui.add(egui::Slider::new(&mut v, min..=max).step_by(step as f64).show_value(false).trailing_fill(true));
                    if resp.dragged() {
                        app.dpi_draft = Some(v);
                    } else if resp.drag_stopped() || resp.changed() {
                        commit_stage(app, i, v, connected);
                    }
                    ui.horizontal(|ui| {
                        ui.set_width(slider_w);
                        ui.label(RichText::new(fmt_thousands(min as u32)).font(mono(11.0)).color(MUTED));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(fmt_thousands(max as u32)).font(mono(11.0)).color(MUTED));
                        });
                    });
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let is_default = app.cfg.default_stage == i;
                    ui.add_enabled_ui(!is_default, |ui| {
                        if button(ui, ButtonKind::Secondary, None, if is_default { "Default stage" } else { "Use as default" }).clicked() {
                            app.cfg.default_stage = i;
                        }
                    });
                    let mut exact = app.cfg.dpi_stages[i];
                    let dv = ui.add(egui::DragValue::new(&mut exact).range(min..=max).speed(step as f64));
                    if dv.changed() && !dv.dragged() || dv.drag_stopped() {
                        let snapped = (exact / step.max(1)) * step.max(1);
                        commit_stage(app, i, snapped.clamp(min, max), connected);
                    }
                });
            });
        });
        caption(ui, "Hidra keeps these five stages and sends the chosen one to the mouse. The G7/G8 buttons still step through the stages stored in the mouse's onboard profile.");
    });
}

fn commit_stage(app: &mut HidraApp, i: usize, v: u16, connected: bool) {
    app.dpi_draft = None;
    if app.cfg.dpi_stages[i] != v {
        app.cfg.dpi_stages[i] = v;
    }
    if connected {
        app.send(Command::SetDpi(v));
    }
}

fn stage_tile(ui: &mut Ui, w: f32, i: usize, v: u16, editing: bool, default: bool, live: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 88.0), Sense::click());
    let p = ui.painter();
    let (fill, border) = if editing {
        (accent_soft(), ACCENT)
    } else if resp.hovered() {
        (Color32::from_rgb(0x20, 0x24, 0x2B), Color32::from_rgb(0x3A, 0x40, 0x4A))
    } else {
        (RAISED, Color32::from_rgb(0x2A, 0x2F, 0x37))
    };
    p.rect(rect, 12, fill, Stroke::new(1.0, border), StrokeKind::Inside);
    p.text(rect.min + vec2(14.0, 22.0), Align2::LEFT_CENTER, format!("Stage {}", i + 1), regular(12.0), SUBTLE);
    let mut x = rect.right() - 12.0;
    for (show, label, fill, color) in [(default, "Default", GREEN_BG, GREEN_TEXT), (live, "Active", accent_soft(), ACCENT)] {
        if show {
            let g = p.layout_no_wrap(label.to_string(), semibold(11.0), color);
            let r = Rect::from_min_size(pos2(x - g.size().x - 12.0, rect.top() + 12.0), vec2(g.size().x + 12.0, 20.0));
            if r.left() < rect.left() + 72.0 {
                break; // no room next to the "Stage n" label
            }
            badge(p, r, label, fill, color, semibold(11.0));
            x = r.left() - 6.0;
        }
    }
    p.text(rect.min + vec2(14.0, 60.0), Align2::LEFT_CENTER, fmt_thousands(v as u32), mono(22.0), TEXT);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

fn pointer_card(app: &mut HidraApp, ui: &mut Ui) {
    card(ui, |ui| {
        section_title(ui, "Pointer");
        ui.add_space(6.0);
        ui.label(RichText::new("Report rate").font(medium(13.0)).color(TEXT));
        let rate = app.snap.as_ref().and_then(|s| s.rate.clone());
        let mut options: Vec<u8> = rate.as_ref().map(|r| r.supported_ms.clone()).unwrap_or_else(|| vec![1, 2, 4, 8]);
        options.sort();
        let labels: Vec<String> = options.iter().map(|ms| format!("{} Hz", 1000 / *ms as u32)).collect();
        let selected = rate.as_ref().and_then(|r| options.iter().position(|&ms| ms == r.current_ms));
        let onboard = app.snap.as_ref().and_then(|s| s.onboard.as_ref()).is_some_and(|o| o.mode == OnboardMode::Onboard);
        let enabled = app.connected() && rate.is_some() && !onboard;
        let clicked = ui.add_enabled_ui(enabled, |ui| segmented(ui, &labels, selected, mono(13.0))).inner;
        if let Some(i) = clicked {
            app.send(Command::SetReportRate(options[i]));
        }
        if onboard {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(RichText::new("The mouse takes its report rate from its onboard profile. Turn onboard memory off on the").font(regular(12.5)).color(SUBTLE));
                if ui.link(RichText::new("Device page").font(regular(12.5))).clicked() {
                    app.page = super::Page::Device;
                }
                ui.label(RichText::new("to change it here.").font(regular(12.5)).color(SUBTLE));
            });
        } else {
            caption(ui, "How often the mouse reports its position. 1000 Hz is smoothest; lower rates save battery.");
        }
        ui.add_space(10.0);

        let Some(desk) = app.desktop.clone() else {
            notice(ui, Tone::Warn, "Acceleration and speed need GNOME's gsettings, which wasn't found.");
            return;
        };
        ui.label(RichText::new("Acceleration").font(medium(13.0)).color(TEXT));
        let modes = [Accel::Flat, Accel::Adaptive, Accel::Default];
        let labels: Vec<String> = ["Flat", "Adaptive", "System default"].map(String::from).to_vec();
        let sel = modes.iter().position(|m| *m == desk.accel);
        if let Some(i) = segmented(ui, &labels, sel, semibold(13.0)) {
            match Desktop::set_accel(modes[i]) {
                Ok(()) => set_desktop(app, |d| d.accel = modes[i]),
                Err(e) => app.toast(format!("Couldn't change acceleration: {e}"), true),
            }
        }
        caption(
            ui,
            match desk.accel {
                Accel::Flat => "Same distance for the same hand movement. Best for games.",
                Accel::Adaptive => "Faster flicks travel further. Comfortable for desktop use.",
                Accel::Default => "Uses the desktop's default (adaptive on GNOME).",
            },
        );
        ui.add_space(10.0);
        ui.label(RichText::new("Pointer speed").font(medium(13.0)).color(TEXT));
        ui.spacing_mut().slider_width = ui.available_width();
        let resp = ui.add(egui::Slider::new(&mut app.speed, -1.0..=1.0).show_value(false).trailing_fill(true));
        if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
            let v = app.speed;
            match Desktop::set_speed(v) {
                Ok(()) => set_desktop(app, |d| d.speed = v),
                Err(e) => app.toast(format!("Couldn't change speed: {e}"), true),
            }
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new("Slower").font(regular(11.0)).color(MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.label(RichText::new("Faster").font(regular(11.0)).color(MUTED)));
        });
        caption(ui, "Desktop setting · applies to every mouse.");
    });
}

fn scroll_card(app: &mut HidraApp, ui: &mut Ui) {
    card(ui, |ui| {
        section_title(ui, "Scrolling");
        ui.add_space(6.0);
        if let Some(desk) = app.desktop.clone() {
            let mut on = desk.natural_scroll;
            if setting_row(ui, "Natural scrolling", "Content moves with the wheel, like a touchpad.", &mut on).changed() {
                match Desktop::set_natural_scroll(on) {
                    Ok(()) => set_desktop(app, |d| d.natural_scroll = on),
                    Err(e) => app.toast(format!("Couldn't change scrolling: {e}"), true),
                }
            }
            caption(ui, "Desktop setting · applies to every mouse.");
        }
        ui.add_space(8.0);
        inset(ui, |ui| {
            ui.horizontal(|ui| {
                icon_widget(ui, Icon::Info, 16.0, SUBTLE);
                wrapped(
                    ui,
                    RichText::new("The wheel switches between ratchet and free-spin with the G9 button behind it. The switch is mechanical, so the app can't read it.")
                        .font(regular(12.5))
                        .color(TEXT_2),
                );
            });
        });
    });
}

fn setting_row(ui: &mut Ui, title: &str, hint: &str, on: &mut bool) -> egui::Response {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - 64.0).max(120.0));
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(RichText::new(title).font(medium(14.0)).color(TEXT));
            ui.label(RichText::new(hint).font(regular(12.0)).color(SUBTLE));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| toggle(ui, on)).inner
    })
    .inner
}

fn set_desktop(app: &mut HidraApp, f: impl FnOnce(&mut Desktop)) {
    if let Some(d) = &mut app.desktop {
        f(d);
    }
}
