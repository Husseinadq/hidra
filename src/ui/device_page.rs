use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, CursorIcon, Layout, Rect, RichText, Sense, Stroke, StrokeKind, Ui};

use super::{theme::*, HidraApp};
use crate::device::{feature, Charging, OnboardMode};
use crate::worker::{Command, Link};

const UDEV_RULE: &str = r#"KERNEL=="hidraw*", ATTRS{idVendor}=="046d", MODE="0660", TAG+="uaccess""#;

pub fn show(app: &mut HidraApp, ui: &mut Ui) {
    page_title(ui, "Device & memory", "Onboard profiles, battery and connection details.");
    ui.add_space(8.0);

    let Some(snap) = app.snap.clone() else {
        empty_state(app, ui);
        return;
    };
    if matches!(app.link, Link::Asleep) {
        notice(ui, Tone::Warn, "The mouse isn't answering. Move it or switch it on; values below are from the last time it was awake.");
        ui.add_space(8.0);
    }

    // Stat tiles.
    let w = ui.available_width();
    let per_row = (((w + 12.0) / 220.0).floor() as usize).clamp(1, 4);
    let tile_w = (w - 12.0 * (per_row as f32 - 1.0)) / per_row as f32;
    let fw = snap.firmware.iter().find(|f| f.kind == "Firmware").map(|f| f.version.clone()).unwrap_or_else(|| "Unknown".into());
    let (batt_value, batt_sub, batt_pct) = match snap.battery {
        Some(b) => (
            format!("{}%", b.percent),
            match b.charging {
                Charging::Charging => "Charging",
                Charging::Full => "Fully charged",
                Charging::Error => "Charging error",
                Charging::Discharging => "On battery",
            }
            .to_string(),
            Some(b.percent),
        ),
        None => ("—".into(), "Wired, no battery".into(), None),
    };
    let dpi_range = snap.dpi.as_ref().map(|d| format!("{} – {} DPI", fmt_thousands(d.min as u32), fmt_thousands(d.max as u32))).unwrap_or_default();
    let tiles = [
        ("Connection", if snap.wireless { "Lightspeed".to_string() } else { "USB cable".to_string() }, snap.path.display().to_string(), None),
        ("Battery", batt_value, batt_sub, batt_pct),
        ("Firmware", fw, format!("HID++ {}.{}", snap.protocol.0, snap.protocol.1), None),
        ("Sensor", "HERO 25K".to_string(), dpi_range, None),
    ];
    for row in tiles.chunks(per_row) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            for (label, value, sub, pct) in row {
                stat_tile(ui, tile_w, label, value, sub, *pct);
            }
        });
    }
    ui.add_space(GAP - 10.0);

    if let Some(o) = snap.onboard.clone() {
        card(ui, |ui| {
            let mut onboard = o.mode == OnboardMode::Onboard;
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((ui.available_width() - 70.0).max(200.0));
                    section_title(ui, "Onboard memory");
                    ui.label(
                        RichText::new(if onboard {
                            "The mouse runs from its own saved profiles, so your layout works on any computer, even without Hidra."
                        } else {
                            "Host mode: the mouse ignores its saved profiles and takes settings live from this computer."
                        })
                        .font(regular(13.0))
                        .color(SUBTLE),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let enabled = app.connected();
                    if ui.add_enabled_ui(enabled, |ui| toggle(ui, &mut onboard)).inner.changed() {
                        app.send(Command::SetOnboardMode(if onboard { OnboardMode::Onboard } else { OnboardMode::Host }));
                    }
                });
            });
            ui.add_space(8.0);

            let count = o.profile_count.max(1) as usize;
            let w = ui.available_width();
            let per_row = (((w + 10.0) / 150.0).floor() as usize).clamp(1, count);
            let tile_w = (w - 10.0 * (per_row as f32 - 1.0)) / per_row as f32;
            let mut picked = None;
            for row in (0..count).collect::<Vec<_>>().chunks(per_row) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for &i in row {
                        let n = i as u16 + 1;
                        let name = app.cfg.slot_names.get(i).cloned().unwrap_or_else(|| format!("Slot {n}"));
                        let in_use = onboard && o.current_profile == n;
                        if slot_tile(ui, tile_w, n, &name, in_use, onboard).clicked() {
                            picked = Some(n);
                        }
                    }
                });
            }
            if let Some(n) = picked {
                if !onboard {
                    app.toast("Turn onboard memory on to switch slots.", true);
                } else if app.connected() {
                    app.send(Command::SetProfile(n));
                }
            }
            ui.add_space(4.0);

            let current = (o.current_profile.max(1) - 1) as usize;
            if app.cfg.slot_names.len() < count {
                app.cfg.slot_names.resize(count, String::new());
            }
            ui.horizontal(|ui| {
                if app.renaming_slot == Some(current) {
                    let r = ui.add(egui::TextEdit::singleline(&mut app.cfg.slot_names[current]).desired_width(220.0).margin(vec2(10.0, 9.0)));
                    r.request_focus();
                    if r.lost_focus() || button(ui, ButtonKind::Primary, Some(Icon::Check), "Done").clicked() {
                        app.renaming_slot = None;
                    }
                } else if button(ui, ButtonKind::Secondary, None, &format!("Rename slot {}", current + 1)).clicked() {
                    app.renaming_slot = Some(current);
                }
            });
            caption(ui, "Slot names are kept by Hidra; the mouse only stores numbers.");
        });
        ui.add_space(GAP - 10.0);
    }

    card(ui, |ui| {
        section_title(ui, "Details");
        ui.add_space(4.0);
        egui::Grid::new("details").num_columns(2).spacing(vec2(24.0, 6.0)).min_row_height(22.0).show(ui, |ui| {
            let row = |ui: &mut Ui, k: &str, v: String| {
                ui.label(RichText::new(k).font(regular(13.0)).color(SUBTLE));
                ui.label(RichText::new(v).font(mono(12.5)).color(TEXT));
                ui.end_row();
            };
            row(ui, "Name", snap.name.clone());
            row(ui, "Device node", snap.path.display().to_string());
            for f in &snap.firmware {
                row(ui, f.kind, f.version.clone());
            }
            if let Some(o) = &snap.onboard {
                row(ui, "Programmable buttons", o.button_count.to_string());
            }
        });
        ui.add_space(6.0);
        egui::CollapsingHeader::new(RichText::new(format!("Supported features ({})", snap.features.len())).font(medium(13.0)).color(TEXT_2))
            .id_salt("features")
            .show(ui, |ui| {
                egui::Grid::new("feature-list").num_columns(3).spacing(vec2(20.0, 4.0)).min_row_height(20.0).show(ui, |ui| {
                    for f in &snap.features {
                        let used = matches!(
                            f.id,
                            feature::UNIFIED_BATTERY | feature::ADJUSTABLE_DPI | feature::REPORT_RATE | feature::ONBOARD_PROFILES | feature::DEVICE_NAME | feature::FIRMWARE_INFO
                        );
                        ui.label(RichText::new(format!("0x{:04X}", f.id)).font(mono(12.0)).color(if used { ACCENT } else { TEXT_2 }));
                        let name = feature::name(f.id);
                        ui.label(RichText::new(if name.is_empty() { "—" } else { name }).font(regular(12.5)).color(if f.hidden { MUTED } else { TEXT_2 }));
                        ui.label(RichText::new(format!("v{}", f.version)).font(mono(11.0)).color(MUTED));
                        ui.end_row();
                    }
                });
                caption(ui, "Highlighted features are the ones Hidra uses today. RGB (0x8071) and button spy (0x8110) are next.");
            });
    });
}

fn empty_state(app: &mut HidraApp, ui: &mut Ui) {
    card(ui, |ui| {
        let (title, body) = match &app.link {
            Link::NoAccess(p) => (
                "Hidra can't open the mouse",
                format!("{} exists, but your user isn't allowed to open it. Install this udev rule once, then unplug the receiver and plug it back in:", p.display()),
            ),
            Link::Asleep => ("The mouse is asleep", "Its receiver is plugged in but the mouse isn't answering. Move it or switch it on.".into()),
            Link::Searching => ("Looking for the mouse…", "Hidra checks for Logitech devices every few seconds.".into()),
            _ => ("No Logitech mouse found", "Plug in the Lightspeed receiver or the USB cable, then press Scan again.".into()),
        };
        section_title(ui, title);
        wrapped(ui, RichText::new(body).font(regular(14.0)).color(TEXT_2));
        if matches!(app.link, Link::NoAccess(_)) {
            ui.add_space(6.0);
            inset(ui, |ui| {
                ui.label(RichText::new("/etc/udev/rules.d/70-hidra.rules").font(mono(12.0)).color(SUBTLE));
                ui.label(RichText::new(UDEV_RULE).font(mono(13.0)).color(TEXT));
            });
            ui.horizontal(|ui| {
                if button(ui, ButtonKind::Secondary, Some(Icon::Copy), "Copy rule").clicked() {
                    ui.ctx().copy_text(UDEV_RULE.to_string());
                    app.toast("Rule copied", false);
                }
                ui.label(RichText::new("then: sudo udevadm control --reload && sudo udevadm trigger").font(mono(12.0)).color(SUBTLE));
            });
        }
        ui.add_space(6.0);
        if button(ui, ButtonKind::Primary, Some(Icon::Refresh), "Scan again").clicked() {
            app.send(Command::Rescan);
        }
    });
}

fn stat_tile(ui: &mut Ui, w: f32, label: &str, value: &str, sub: &str, pct: Option<u8>) {
    let (rect, _) = ui.allocate_exact_size(vec2(w, 96.0), Sense::hover());
    let p = ui.painter();
    p.rect(rect, 14, CARD, Stroke::new(1.0, CARD_BORDER), StrokeKind::Inside);
    p.text(rect.min + vec2(18.0, 22.0), Align2::LEFT_CENTER, label, regular(12.0), SUBTLE);
    p.text(rect.min + vec2(18.0, 48.0), Align2::LEFT_CENTER, value, semibold(18.0), TEXT);
    let sub_font = if sub.starts_with('/') { mono(11.5) } else { regular(12.0) };
    p.text(rect.min + vec2(18.0, 74.0), Align2::LEFT_CENTER, sub, sub_font, MUTED);
    if let Some(pct) = pct {
        let bar = Rect::from_min_size(pos2(rect.right() - 78.0, rect.top() + 44.0), vec2(56.0, 8.0));
        p.rect_filled(bar, 4, BADGE);
        let fill = Rect::from_min_size(bar.min, vec2(bar.width() * pct as f32 / 100.0, bar.height()));
        p.rect_filled(fill, 4, if pct <= 15 { AMBER } else { GREEN });
    }
}

fn slot_tile(ui: &mut Ui, w: f32, n: u16, name: &str, in_use: bool, enabled: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 84.0), Sense::click());
    let p = ui.painter();
    let (fill, border) = if in_use {
        (accent_soft(), ACCENT)
    } else if resp.hovered() && enabled {
        (Color32::from_rgb(0x20, 0x24, 0x2B), Color32::from_rgb(0x3A, 0x40, 0x4A))
    } else {
        (RAISED, Color32::from_rgb(0x2A, 0x2F, 0x37))
    };
    let alpha = if enabled { 1.0 } else { 0.55 };
    p.rect(rect, 12, fill.gamma_multiply(alpha), Stroke::new(1.0, border.gamma_multiply(alpha)), StrokeKind::Inside);
    p.text(rect.min + vec2(14.0, 22.0), Align2::LEFT_CENTER, format!("Slot {n}"), mono(12.0), SUBTLE);
    if in_use {
        let g = p.layout_no_wrap("In use".into(), semibold(11.0), GREEN_TEXT);
        let r = Rect::from_min_size(pos2(rect.right() - g.size().x - 24.0, rect.top() + 12.0), vec2(g.size().x + 12.0, 20.0));
        badge(p, r, "In use", GREEN_BG, GREEN_TEXT, semibold(11.0));
    }
    let shown = if name.trim().is_empty() { format!("Slot {n}") } else { name.to_string() };
    p.text(rect.min + vec2(14.0, 56.0), Align2::LEFT_CENTER, shown, semibold(15.0), TEXT.gamma_multiply(alpha));
    if enabled {
        resp.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        resp
    }
}
