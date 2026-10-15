//! Background thread that owns the mouse. The UI sends `Command`s and gets
//! `Update`s back; nothing in the UI thread ever blocks on the device.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::device::{self, feature, Battery, Device, DpiCaps, FeatureEntry, Firmware, Onboard, OnboardMode, ReportRate};
use crate::hidpp::Error;
use crate::profile::{self, Raw};

#[derive(Debug, Clone)]
pub enum Command {
    Rescan,
    SetDpi(u16),
    SetReportRate(u8),
    SetOnboardMode(OnboardMode),
    SetProfile(u16),
    /// Store bindings (button table index, binding) in the active onboard
    /// profile. The string describes the change for the confirmation.
    WriteButtons(Vec<(usize, Raw)>, String),
}

/// The button table of the onboard profile the mouse is using.
#[derive(Debug, Clone)]
pub struct ProfileButtons {
    pub sector: u16,
    pub raws: Vec<Raw>,
}

/// Everything the UI shows about the connected mouse.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub name: String,
    pub path: PathBuf,
    pub protocol: (u8, u8),
    pub wireless: bool,
    pub firmware: Vec<Firmware>,
    pub battery: Option<Battery>,
    pub dpi: Option<DpiCaps>,
    pub rate: Option<ReportRate>,
    pub onboard: Option<Onboard>,
    pub buttons: Option<ProfileButtons>,
    pub features: Vec<FeatureEntry>,
}

#[derive(Debug, Clone)]
pub enum Link {
    Searching,
    /// Logitech nodes exist but none could be opened.
    NoAccess(PathBuf),
    NotFound,
    Connected,
    /// Paired but not answering (switched off or asleep).
    Asleep,
}

#[derive(Debug, Clone)]
pub enum Update {
    Link(Link),
    Snapshot(Box<Snapshot>),
    Battery(Battery),
    Dpi(u16),
    Onboard(Onboard),
    Rate(ReportRate),
    /// Fresh button table, with a confirmation message after a write.
    Buttons(ProfileButtons, Option<String>),
    Error(String),
}

pub struct Handle {
    pub tx: Sender<Command>,
    pub rx: Receiver<Update>,
}

pub fn spawn(repaint: impl Fn() + Send + 'static) -> Handle {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (up_tx, up_rx) = mpsc::channel();
    thread::Builder::new()
        .name("hidra-device".into())
        .spawn(move || {
            Worker { rx: cmd_rx, tx: up_tx, repaint: Box::new(repaint), dev: None, asleep: false, last_wake_try: Instant::now() }.run()
        })
        .expect("spawn device thread");
    Handle { tx: cmd_tx, rx: up_rx }
}

struct Worker {
    rx: Receiver<Command>,
    tx: Sender<Update>,
    repaint: Box<dyn Fn() + Send>,
    dev: Option<Device>,
    asleep: bool,
    last_wake_try: Instant,
}

const WAKE_RETRY: Duration = Duration::from_secs(10);
const BATTERY_POLL: Duration = Duration::from_secs(60);
const RESCAN: Duration = Duration::from_secs(3);

impl Worker {
    fn send(&self, u: Update) {
        let _ = self.tx.send(u);
        (self.repaint)();
    }

    fn run(mut self) {
        let mut last_battery = Instant::now();
        let mut last_scan = Instant::now() - RESCAN;
        self.send(Update::Link(Link::Searching));
        loop {
            if self.dev.is_none() && last_scan.elapsed() >= RESCAN {
                last_scan = Instant::now();
                self.connect();
                last_battery = Instant::now();
            }

            match self.rx.recv_timeout(Duration::from_millis(150)) {
                Ok(cmd) => self.handle(cmd),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }

            if self.dev.is_some() {
                self.drain_notifications();
                if self.asleep && self.last_wake_try.elapsed() >= WAKE_RETRY {
                    self.refresh();
                } else if !self.asleep && last_battery.elapsed() >= BATTERY_POLL {
                    last_battery = Instant::now();
                    self.poll_battery();
                }
            }
        }
    }

    fn connect(&mut self) {
        let candidates = device::candidates();
        if candidates.is_empty() {
            self.send(Update::Link(Link::NotFound));
            return;
        }
        let mut denied = None;
        for c in &candidates {
            match Device::open(&c.path) {
                Ok(dev) => {
                    self.dev = Some(dev);
                    self.refresh();
                    return;
                }
                Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::PermissionDenied => denied = Some(c.path.clone()),
                Err(Error::Unreachable) => {
                    // Paired, but switched off. Keep looking for others.
                }
                Err(_) => {}
            }
        }
        self.send(Update::Link(match denied {
            Some(p) => Link::NoAccess(p),
            None if candidates.iter().any(|c| c.product_id != 0) => Link::Asleep,
            None => Link::NotFound,
        }));
    }

    fn read_all(&mut self) -> Result<Snapshot, Error> {
        let dev = self.dev.as_mut().expect("device");
        let features = dev.features()?;
        let has = |id| features.iter().any(|f| f.id == id);
        let mut s = Snapshot {
            path: dev.path.clone(),
            protocol: dev.protocol,
            wireless: has(feature::WIRELESS_STATUS),
            name: dev.name().unwrap_or_else(|_| "Logitech mouse".into()),
            firmware: dev.firmware().unwrap_or_default(),
            ..Default::default()
        };
        if has(feature::UNIFIED_BATTERY) {
            s.battery = dev.battery().ok();
        }
        if has(feature::ADJUSTABLE_DPI) {
            s.dpi = dev.dpi().ok();
        }
        if has(feature::REPORT_RATE) {
            s.rate = dev.report_rate().ok();
        }
        if has(feature::ONBOARD_PROFILES) {
            s.onboard = dev.onboard().ok();
            if let Some(o) = &s.onboard {
                s.buttons = read_buttons(dev, o).ok();
            }
        }
        s.features = features;
        Ok(s)
    }

    fn handle(&mut self, cmd: Command) {
        if let Command::Rescan = cmd {
            self.dev = None;
            self.connect();
            return;
        }
        let Some(dev) = self.dev.as_mut() else {
            self.send(Update::Error("No mouse connected".into()));
            return;
        };
        let result = match cmd {
            Command::Rescan => unreachable!(),
            Command::SetDpi(v) => dev.set_dpi(v).and_then(|_| dev.dpi()).map(|d| vec![Update::Dpi(d.current)]),
            Command::SetReportRate(ms) => dev.set_report_rate(ms).and_then(|_| dev.report_rate()).map(|r| vec![Update::Rate(r)]),
            Command::SetOnboardMode(m) => dev.set_onboard_mode(m).and_then(|_| onboard_and_buttons(dev)),
            Command::SetProfile(p) => dev.set_current_profile(p).and_then(|_| onboard_and_buttons(dev)),
            Command::WriteButtons(changes, what) => match write_buttons(dev, &changes) {
                Ok(b) => Ok(vec![Update::Buttons(b, Some(what))]),
                Err(WriteError::Hidpp(e)) => Err(e),
                Err(WriteError::Refused(msg)) => Ok(vec![Update::Error(msg)]),
            },
        };
        match result {
            Ok(updates) => {
                for u in updates {
                    self.send(u);
                }
            }
            Err(Error::Device(2 | 9)) => self.send(Update::Error("The mouse refused that in its current mode. Try turning onboard memory off.".into())),
            Err(e) => self.fail(e),
        }
    }

    /// Re-reads everything; used on connect and when the mouse wakes up.
    fn refresh(&mut self) {
        self.last_wake_try = Instant::now();
        match self.read_all() {
            Ok(snap) => {
                self.asleep = false;
                self.send(Update::Link(Link::Connected));
                self.send(Update::Snapshot(Box::new(snap)));
            }
            Err(e) => self.fail(e),
        }
    }

    fn poll_battery(&mut self) {
        let Some(dev) = self.dev.as_mut() else { return };
        if !dev.has(feature::UNIFIED_BATTERY) {
            return;
        }
        match dev.battery() {
            Ok(b) => self.send(Update::Battery(b)),
            Err(e) => self.fail(e),
        }
    }

    fn drain_notifications(&mut self) {
        let mut reconnected = false;
        loop {
            let Some(dev) = self.dev.as_mut() else { return };
            let n = match dev.transport().next_notification(Duration::ZERO) {
                Ok(Some(n)) => n,
                Ok(None) => break,
                Err(e) => return self.fail(e),
            };
            match dev.feature_at(n.feature_index) {
                Some(feature::UNIFIED_BATTERY) => self.send(Update::Battery(device::parse_battery(&n.data))),
                Some(feature::WIRELESS_STATUS) => reconnected = true,
                _ => {}
            }
        }
        if reconnected {
            // Settings may have changed while the mouse was off.
            self.refresh();
        }
    }

    fn fail(&mut self, e: Error) {
        match e {
            Error::Io(_) => {
                self.dev = None;
                self.asleep = false;
                self.send(Update::Link(Link::Searching));
            }
            Error::Unreachable | Error::Timeout => {
                self.asleep = true;
                self.send(Update::Link(Link::Asleep));
            }
            other => self.send(Update::Error(other.to_string())),
        }
    }
}

fn onboard_and_buttons(dev: &mut Device) -> Result<Vec<Update>, Error> {
    let o = dev.onboard()?;
    let mut out = Vec::new();
    if let Ok(b) = read_buttons(dev, &o) {
        out.push(Update::Buttons(b, None));
    }
    out.insert(0, Update::Onboard(o));
    Ok(out)
}

pub fn read_buttons(dev: &mut Device, o: &Onboard) -> Result<ProfileButtons, Error> {
    let sector = o.current_profile;
    let data = dev.read_sector(sector, o.sector_size)?;
    Ok(ProfileButtons { sector, raws: profile::buttons(&data) })
}

pub enum WriteError {
    Hidpp(Error),
    Refused(String),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::Hidpp(e) => write!(f, "{e}"),
            WriteError::Refused(m) => write!(f, "{m}"),
        }
    }
}

impl From<Error> for WriteError {
    fn from(e: Error) -> Self {
        WriteError::Hidpp(e)
    }
}

/// Read-modify-write of the active profile sector, with a one-time backup
/// of the original and a read-back check.
pub fn write_buttons(dev: &mut Device, changes: &[(usize, Raw)]) -> Result<ProfileButtons, WriteError> {
    let o = dev.onboard()?;
    let sector = o.current_profile;
    if sector == 0 || sector >= 0x100 {
        return Err(WriteError::Refused("The mouse is using a built-in profile that can't be changed. Pick a profile slot in the top bar first.".into()));
    }
    let mut data = dev.read_sector(sector, o.sector_size)?;
    if !profile::crc_ok(&data) {
        return Err(WriteError::Refused("The profile on the mouse looks damaged, so Hidra won't overwrite it.".into()));
    }
    backup(sector, &data);
    let before = data.clone();
    for &(i, raw) in changes {
        let off = profile::BUTTONS_OFFSET + i * 4;
        if i < profile::BUTTON_COUNT && off + 4 <= data.len() - 2 {
            data[off..off + 4].copy_from_slice(&raw);
        }
    }
    if data != before {
        profile::seal(&mut data);
        dev.write_sector(sector, &data)?;
        let back = dev.read_sector(sector, o.sector_size)?;
        if back != data {
            return Err(WriteError::Refused("The mouse didn't keep the change. Try again.".into()));
        }
        if o.mode == OnboardMode::Onboard {
            // Re-select the profile so the mouse reloads it.
            dev.set_current_profile(sector)?;
        }
    }
    Ok(ProfileButtons { sector, raws: profile::buttons(&data) })
}

/// Keeps the first copy of each profile sector Hidra ever changes.
fn backup(sector: u16, data: &[u8]) {
    let Some(dir) = crate::config::dir().map(|d| d.join("backup")) else { return };
    let file = dir.join(format!("sector_{sector:04x}.bin"));
    if file.exists() {
        return;
    }
    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = std::fs::write(file, data);
    }
}
