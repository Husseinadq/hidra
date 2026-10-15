mod buttons;
mod device_page;
mod mouse_art;
mod pointer;
mod profiles;
pub mod theme;

use std::time::{Duration, Instant};

use eframe::egui::{
    self, pos2, vec2, Align, Align2, Color32, CursorIcon, Frame, Id, Layout, Margin, Rect, ResizeDirection, RichText, Sense, Stroke, StrokeKind,
    Ui, ViewportCommand,
};

use crate::config::Config;
use crate::desktop::Desktop;
use crate::device::{Charging, OnboardMode};
use crate::worker::{self, Command, Link, Snapshot, Update};
use theme::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Buttons,
    Pointer,
    Profiles,
    Device,
}

struct Toast {
    text: String,
    error: bool,
    at: Instant,
}

pub struct HidraApp {
    worker: worker::Handle,
    link: Link,
    snap: Option<Snapshot>,
    cfg: Config,
    saved: Config,
    desktop: Option<Desktop>,
    page: Page,
    toasts: Vec<Toast>,
    // per-page view state
    sel_control: usize,
    edit_stage: usize,
    sel_app: usize,
    renaming_slot: Option<usize>,
    dpi_draft: Option<u16>,
    speed: f64,
    shots: Option<Screenshots>,
}

/// Developer aid: with HIDRA_SCREENSHOT_DIR set, renders each page, saves
/// it as a PPM image and quits. Never sends anything to the mouse.
struct Screenshots {
    dir: std::path::PathBuf,
    pages: Vec<(Page, &'static str)>,
    frames: u32,
    waiting: bool,
}

impl HidraApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        let ctx = cc.egui_ctx.clone();
        let worker = worker::spawn(move || ctx.request_repaint());
        let cfg = Config::load();
        let desktop = Desktop::read();
        Self {
            worker,
            link: Link::Searching,
            snap: None,
            saved: cfg.clone(),
            cfg,
            speed: desktop.as_ref().map(|d| d.speed).unwrap_or(0.0),
            desktop,
            page: Page::Buttons,
            toasts: Vec::new(),
            sel_control: 8,
            edit_stage: 2,
            sel_app: 0,
            renaming_slot: None,
            dpi_draft: None,
            shots: std::env::var_os("HIDRA_SCREENSHOT_DIR").map(|d| Screenshots {
                dir: d.into(),
                pages: vec![(Page::Buttons, "buttons"), (Page::Pointer, "pointer"), (Page::Profiles, "profiles"), (Page::Device, "device")],
                frames: 0,
                waiting: false,
            }),
        }
    }

    fn send(&self, cmd: Command) {
        let _ = self.worker.tx.send(cmd);
    }

    fn toast(&mut self, text: impl Into<String>, error: bool) {
        self.toasts.push(Toast { text: text.into(), error, at: Instant::now() });
    }

    fn connected(&self) -> bool {
        matches!(self.link, Link::Connected) && self.snap.is_some()
    }

    fn apply(&mut self, u: Update) {
        match u {
            Update::Link(l) => self.link = l,
            Update::Snapshot(s) => {
                if let Some(d) = &s.dpi {
                    // Show the stage that matches what the mouse is using.
                    if let Some(i) = self.cfg.dpi_stages.iter().position(|&v| v == d.current) {
                        self.edit_stage = i;
                    }
                }
                self.snap = Some(*s);
            }
            Update::Battery(b) => {
                if let Some(s) = &mut self.snap {
                    s.battery = Some(b);
                }
            }
            Update::Dpi(v) => {
                if let Some(d) = self.snap.as_mut().and_then(|s| s.dpi.as_mut()) {
                    d.current = v;
                }
            }
            Update::Rate(r) => {
                let hz = 1000 / r.current_ms.max(1) as u32;
                if let Some(s) = &mut self.snap {
                    s.rate = Some(r);
                }
                self.toast(format!("Report rate set to {hz} Hz"), false);
            }
            Update::Onboard(o) => {
                if let Some(s) = &mut self.snap {
                    s.onboard = Some(o);
                }
            }
            Update::Error(e) => self.toast(e, true),
        }
    }

    fn header(&mut self, ui: &mut Ui) {
        egui::Panel::top("header")
            .frame(Frame::new().fill(HEADER).inner_margin(Margin { left: 18, right: 12, top: 10, bottom: 10 }))
            .show_separator_line(true)
            .show(ui, |ui| {
                // Dragging empty header space moves the window.
                let bar = ui.interact(ui.max_rect(), Id::new("titlebar"), Sense::click_and_drag());
                let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                if bar.drag_started() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if bar.double_clicked() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                }
                ui.horizontal(|ui| {
                    ui.set_height(34.0);
                    icon_widget(ui, Icon::Logo, 22.0, ACCENT);
                    ui.label(RichText::new("Hidra").font(semibold(15.0)).color(TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, Icon::Close, "Close").clicked() {
                            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                        }
                        if icon_button(ui, Icon::Square, if maximized { "Restore" } else { "Maximize" }).clicked() {
                            ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                        }
                        if icon_button(ui, Icon::Minus, "Minimize").clicked() {
                            ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                        }
                        ui.add_space(10.0);
                        self.profile_picker(ui);
                    });
                });
            });
    }

    /// Header drop-down for the onboard profile slot in use.
    fn profile_picker(&mut self, ui: &mut Ui) {
        let Some(o) = self.snap.as_ref().and_then(|s| s.onboard.clone()) else { return };
        let onboard = o.mode == OnboardMode::Onboard && self.connected();
        let name = |i: u16| format!("{} · {}", i, self.cfg.slot_names.get(i as usize - 1).map(String::as_str).unwrap_or("Slot"));
        let current = if onboard { name(o.current_profile) } else { "Host mode".to_string() };
        let mut pick = None;
        ui.add_enabled_ui(onboard, |ui| {
            egui::ComboBox::from_id_salt("profile")
                .selected_text(RichText::new(current).font(medium(13.0)).color(TEXT))
                .width(150.0)
                .show_ui(ui, |ui| {
                    for i in 1..=o.profile_count as u16 {
                        if ui.selectable_label(i == o.current_profile, name(i)).clicked() {
                            pick = Some(i);
                        }
                    }
                });
        });
        ui.label(RichText::new("Onboard profile").font(regular(13.0)).color(SUBTLE));
        if let Some(i) = pick {
            self.send(Command::SetProfile(i));
        }
    }

    fn sidebar(&mut self, ui: &mut Ui) {
        egui::Panel::left("sidebar")
            .exact_size(250.0)
            .resizable(false)
            .frame(Frame::new().fill(SIDEBAR).inner_margin(Margin::symmetric(14, 20)))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(6.0);
                    overline(ui, "Devices");
                });
                if self.device_card(ui).clicked() {
                    self.page = Page::Device;
                }
                if button(ui, ButtonKind::Secondary, Some(Icon::Refresh), "Scan again").clicked() {
                    self.send(Command::Rescan);
                }
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.add_space(6.0);
                    overline(ui, "Configure");
                });
                for (page, ic, label) in [
                    (Page::Buttons, Icon::Mouse, "Buttons"),
                    (Page::Pointer, Icon::Gauge, "Pointer & scroll"),
                    (Page::Profiles, Icon::Grid, "App profiles"),
                    (Page::Device, Icon::Chip, "Device & memory"),
                ] {
                    if nav_item(ui, ic, label, self.page == page).clicked() {
                        self.page = page;
                    }
                }
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| self.access_box(ui));
            });
    }

    fn device_card(&self, ui: &mut Ui) -> egui::Response {
        let w = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(vec2(w, 66.0), Sense::click());
        let p = ui.painter();
        let border = if resp.hovered() { Color32::from_rgb(0x44, 0x4B, 0x56) } else { BORDER };
        p.rect(rect, 12, Color32::from_rgb(0x1D, 0x21, 0x27), Stroke::new(1.0, border), StrokeKind::Inside);
        let tile = Rect::from_min_size(rect.min + vec2(12.0, 13.0), vec2(40.0, 40.0));
        p.rect_filled(tile, 10, BADGE);
        icon(p, tile.center(), 20.0, Icon::Mouse, ACCENT);
        let name = self.snap.as_ref().map(|s| s.name.as_str()).unwrap_or("Logitech mouse");
        p.text(pos2(tile.right() + 12.0, rect.top() + 24.0), Align2::LEFT_CENTER, name, semibold(14.0), TEXT);
        let (dot, status) = self.status_line();
        status_dot(p, pos2(tile.right() + 15.5, rect.top() + 44.0), dot);
        p.text(pos2(tile.right() + 24.0, rect.top() + 44.0), Align2::LEFT_CENTER, status, regular(12.0), SUBTLE);
        resp.on_hover_cursor(CursorIcon::PointingHand)
    }

    fn status_line(&self) -> (Color32, String) {
        match &self.link {
            Link::Connected => {
                let s = self.snap.as_ref();
                let via = if s.map(|s| s.wireless).unwrap_or(false) { "Lightspeed" } else { "USB" };
                match s.and_then(|s| s.battery) {
                    Some(b) => {
                        let charging = if b.charging == Charging::Charging { " · charging" } else { "" };
                        (if b.percent <= 15 { AMBER } else { GREEN }, format!("{via} · {}%{charging}", b.percent))
                    }
                    None => (GREEN, via.to_string()),
                }
            }
            Link::Asleep => (AMBER, "Asleep — move the mouse".into()),
            Link::Searching => (MUTED, "Looking for the mouse…".into()),
            Link::NotFound => (MUTED, "No mouse found".into()),
            Link::NoAccess(_) => (AMBER, "No permission".into()),
        }
    }

    fn access_box(&self, ui: &mut Ui) {
        let (ok, line1, line2) = match (&self.link, &self.snap) {
            (Link::NoAccess(p), _) => (false, "Device access needed".to_string(), p.display().to_string()),
            (_, Some(s)) => (true, format!("HID++ {}.{}", s.protocol.0, s.protocol.1), s.path.display().to_string()),
            _ => (false, "Not connected".to_string(), "—".to_string()),
        };
        Frame::new().fill(CARD).stroke(Stroke::new(1.0, DIVIDER)).corner_radius(10).inner_margin(Margin::same(12)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 2.0;
            // The sidebar lays this box out bottom-up, so the last line comes first.
            ui.label(RichText::new(line2).font(mono(11.0)).color(MUTED));
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(8.0, 14.0), Sense::hover());
                status_dot(ui.painter(), r.center(), if ok { GREEN } else { MUTED });
                ui.label(RichText::new(line1).font(regular(12.0)).color(TEXT_2));
            });
        });
    }

    fn toasts(&mut self, ui: &mut Ui) {
        self.toasts.retain(|t| t.at.elapsed() < Duration::from_secs(5));
        if self.toasts.is_empty() {
            return;
        }
        ui.ctx().request_repaint_after(Duration::from_millis(250));
        let screen = ui.ctx().content_rect();
        egui::Area::new(Id::new("toasts"))
            .order(egui::Order::Foreground)
            .anchor(Align2::RIGHT_BOTTOM, vec2(-20.0, -20.0))
            .constrain_to(screen)
            .show(ui.ctx(), |ui| {
                for t in &self.toasts {
                    let border = if t.error { DANGER_BORDER } else { Color32::from_rgb(0x2A, 0x3A, 0x4C) };
                    Frame::new().fill(RAISED).stroke(Stroke::new(1.0, border)).corner_radius(10).inner_margin(Margin::symmetric(14, 10)).show(ui, |ui| {
                        ui.set_max_width(360.0);
                        ui.horizontal(|ui| {
                            icon_widget(ui, if t.error { Icon::Info } else { Icon::Check }, 15.0, if t.error { DANGER } else { GREEN });
                            wrapped(ui, RichText::new(&t.text).font(regular(13.0)).color(TEXT));
                        });
                    });
                }
            });
    }

    /// Invisible strips along the window edges for resizing, since the
    /// window draws its own title bar.
    fn resize_edges(&self, ui: &mut Ui) {
        if ui.input(|i| i.viewport().maximized.unwrap_or(false)) {
            return;
        }
        let r = ui.ctx().content_rect();
        let e = 6.0;
        let zones = [
            (Rect::from_min_max(r.left_top(), pos2(r.left() + e, r.bottom())), ResizeDirection::West, CursorIcon::ResizeWest),
            (Rect::from_min_max(pos2(r.right() - e, r.top()), r.right_bottom()), ResizeDirection::East, CursorIcon::ResizeEast),
            (Rect::from_min_max(pos2(r.left(), r.bottom() - e), r.right_bottom()), ResizeDirection::South, CursorIcon::ResizeSouth),
            (Rect::from_min_max(r.left_top(), pos2(r.right(), r.top() + 3.0)), ResizeDirection::North, CursorIcon::ResizeNorth),
            (Rect::from_min_max(pos2(r.right() - 14.0, r.bottom() - 14.0), r.right_bottom()), ResizeDirection::SouthEast, CursorIcon::ResizeSouthEast),
            (Rect::from_min_max(pos2(r.left(), r.bottom() - 14.0), pos2(r.left() + 14.0, r.bottom())), ResizeDirection::SouthWest, CursorIcon::ResizeSouthWest),
        ];
        for (i, (rect, dir, cursor)) in zones.into_iter().enumerate() {
            let resp = ui.interact(rect, Id::new(("resize", i)), Sense::drag()).on_hover_cursor(cursor);
            if resp.drag_started() {
                ui.ctx().send_viewport_cmd(ViewportCommand::BeginResize(dir));
            }
        }
    }
}

impl HidraApp {
    fn screenshots(&mut self, ctx: &egui::Context) {
        let Some(shots) = &mut self.shots else { return };
        let Some(&(page, name)) = shots.pages.first() else {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        };
        self.page = page;
        ctx.request_repaint();
        if shots.waiting {
            let image = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(img) = image {
                let [w, h] = img.size;
                let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
                for px in &img.pixels {
                    out.extend_from_slice(&[px.r(), px.g(), px.b()]);
                }
                let _ = std::fs::write(shots.dir.join(format!("{name}.ppm")), out);
                shots.pages.remove(0);
                shots.frames = 0;
                shots.waiting = false;
            }
            return;
        }
        shots.frames += 1;
        // Give the device a moment to report in before the first shot.
        let wait = if name == "buttons" { 90 } else { 10 };
        if shots.frames >= wait {
            shots.waiting = true;
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
        }
    }
}

fn nav_item(ui: &mut Ui, ic: Icon, label: &str, active: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
    let p = ui.painter();
    if active {
        p.rect_filled(rect, 10, accent_soft());
    } else if resp.hovered() {
        p.rect_filled(rect, 10, Color32::from_rgb(0x1A, 0x1E, 0x24));
    }
    let color = if active { ACCENT } else { TEXT_2 };
    icon(p, pos2(rect.left() + 21.0, rect.center().y), 18.0, ic, color);
    let font = if active { semibold(14.0) } else { regular(14.0) };
    p.text(pos2(rect.left() + 42.0, rect.center().y), Align2::LEFT_CENTER, label, font, color);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

impl eframe::App for HidraApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(u) = self.worker.rx.try_recv() {
            self.apply(u);
        }
        self.screenshots(_ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.header(ui);
        self.sidebar(ui);
        egui::CentralPanel::default().frame(Frame::new().fill(BG)).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                Frame::new().inner_margin(Margin { left: 32, right: 32, top: 28, bottom: 40 }).show(ui, |ui| {
                    let w = ui.available_width().min(1180.0);
                    ui.set_max_width(w);
                    ui.spacing_mut().item_spacing.y = 10.0;
                    match self.page {
                        Page::Buttons => buttons::show(self, ui),
                        Page::Pointer => pointer::show(self, ui),
                        Page::Profiles => profiles::show(self, ui),
                        Page::Device => device_page::show(self, ui),
                    }
                });
            });
        });
        self.toasts(ui);
        self.resize_edges(ui);

        if self.cfg != self.saved {
            match self.cfg.save() {
                Ok(()) => self.saved = self.cfg.clone(),
                Err(e) => {
                    self.saved = self.cfg.clone();
                    self.toast(format!("Couldn't save settings: {e}"), true);
                }
            }
        }
    }
}
