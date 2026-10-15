//! High-level access to the HID++ 2.0 features Hidra uses, plus discovery
//! of Logitech hidraw nodes.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::hidpp::{Error, Result, Transport};

pub mod feature {
    pub const ROOT: u16 = 0x0000;
    pub const FEATURE_SET: u16 = 0x0001;
    pub const FIRMWARE_INFO: u16 = 0x0003;
    pub const DEVICE_NAME: u16 = 0x0005;
    pub const WIRELESS_STATUS: u16 = 0x1D4B;
    pub const UNIFIED_BATTERY: u16 = 0x1004;
    pub const ADJUSTABLE_DPI: u16 = 0x2201;
    pub const ONBOARD_PROFILES: u16 = 0x8100;
    pub const REPORT_RATE: u16 = 0x8060;

    pub fn name(id: u16) -> &'static str {
        match id {
            0x0000 => "Root",
            0x0001 => "Feature set",
            0x0003 => "Firmware info",
            0x0005 => "Device name",
            0x0020 => "Config change",
            0x00D0 => "Firmware update",
            0x1004 => "Unified battery",
            0x1500 => "Force pairing",
            0x1802 => "Device reset",
            0x1803 => "GPIO access",
            0x1806 => "Device properties",
            0x1811 => "OOB state",
            0x1830 => "Power modes",
            0x18A1 => "LED control",
            0x1D4B => "Wireless status",
            0x2121 => "Hi-res wheel",
            0x2201 => "Adjustable DPI",
            0x8060 => "Report rate",
            0x8071 => "RGB effects",
            0x8081 => "Per-key lighting",
            0x8100 => "Onboard profiles",
            0x8110 => "Mouse button spy",
            _ => "",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FeatureEntry {
    pub id: u16,
    pub version: u8,
    pub hidden: bool,
}

#[derive(Debug, Clone)]
pub struct Firmware {
    pub kind: &'static str,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Charging {
    Discharging,
    Charging,
    Full,
    Error,
}

#[derive(Debug, Clone, Copy)]
pub struct Battery {
    pub percent: u8,
    pub charging: Charging,
}

#[derive(Debug, Clone)]
pub struct DpiCaps {
    pub min: u16,
    pub max: u16,
    pub step: u16,
    pub current: u16,
    pub default: u16,
}

#[derive(Debug, Clone)]
pub struct ReportRate {
    /// Current interval in milliseconds.
    pub current_ms: u8,
    /// Supported intervals in milliseconds, fastest first.
    pub supported_ms: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardMode {
    Onboard,
    Host,
}

#[derive(Debug, Clone)]
pub struct Onboard {
    pub mode: OnboardMode,
    pub profile_count: u8,
    pub current_profile: u16,
    pub button_count: u8,
    pub sector_size: u16,
}

pub struct Device {
    t: Transport,
    pub path: PathBuf,
    pub protocol: (u8, u8),
    index: HashMap<u16, u8>,
}

impl Device {
    pub fn open(path: &Path) -> Result<Self> {
        let mut t = Transport::open(path)?;
        // Root.getProtocolVersion with a ping byte.
        let r = t.request(0x00, 1, &[0, 0, 0xAA])?;
        Ok(Self { t, path: path.to_owned(), protocol: (r[0], r[1]), index: HashMap::new() })
    }

    pub fn transport(&mut self) -> &mut Transport {
        &mut self.t
    }

    /// Feature index for a feature id, cached. `None` if unsupported.
    pub fn feature_index(&mut self, id: u16) -> Result<Option<u8>> {
        if let Some(&i) = self.index.get(&id) {
            return Ok((i != 0 || id == feature::ROOT).then_some(i));
        }
        let r = self.t.request(0x00, 0, &id.to_be_bytes())?;
        self.index.insert(id, r[0]);
        Ok((r[0] != 0 || id == feature::ROOT).then_some(r[0]))
    }

    /// Feature id behind an index, from the cache filled by lookups.
    pub fn feature_at(&self, index: u8) -> Option<u16> {
        self.index.iter().find(|(_, &i)| i == index).map(|(&id, _)| id)
    }

    fn call(&mut self, id: u16, function: u8, params: &[u8]) -> Result<[u8; 16]> {
        let idx = self.feature_index(id)?.ok_or(Error::Unsupported(id))?;
        self.t.request(idx, function, params)
    }

    pub fn has(&mut self, id: u16) -> bool {
        matches!(self.feature_index(id), Ok(Some(_)))
    }

    pub fn features(&mut self) -> Result<Vec<FeatureEntry>> {
        let count = self.call(feature::FEATURE_SET, 0, &[])?[0];
        let mut out = Vec::with_capacity(count as usize + 1);
        for i in 0..=count {
            let r = self.call(feature::FEATURE_SET, 1, &[i])?;
            let id = u16::from_be_bytes([r[0], r[1]]);
            self.index.entry(id).or_insert(i);
            out.push(FeatureEntry { id, version: r[3], hidden: r[2] & 0x40 != 0 });
        }
        Ok(out)
    }

    pub fn name(&mut self) -> Result<String> {
        let len = self.call(feature::DEVICE_NAME, 0, &[])?[0] as usize;
        let mut name = Vec::with_capacity(len);
        while name.len() < len {
            let r = self.call(feature::DEVICE_NAME, 1, &[name.len() as u8])?;
            let chunk: Vec<u8> = r.iter().copied().take_while(|&b| b != 0).collect();
            if chunk.is_empty() {
                break;
            }
            name.extend(chunk);
        }
        name.truncate(len);
        Ok(String::from_utf8_lossy(&name).trim().to_string())
    }

    pub fn firmware(&mut self) -> Result<Vec<Firmware>> {
        let count = self.call(feature::FIRMWARE_INFO, 0, &[])?[0];
        let mut out = Vec::new();
        for i in 0..count {
            let r = self.call(feature::FIRMWARE_INFO, 1, &[i])?;
            let kind = match r[0] {
                0 => "Firmware",
                1 => "Bootloader",
                2 => "Hardware",
                4 => "Sensor",
                5 => "Radio",
                _ => continue,
            };
            let prefix: String = r[1..4].iter().filter(|b| b.is_ascii_graphic()).map(|&b| b as char).collect();
            let build = u16::from_be_bytes([r[6], r[7]]);
            out.push(Firmware { kind, version: format!("{prefix}{:02X}.{:02X}_B{build:04X}", r[4], r[5]) });
        }
        Ok(out)
    }

    pub fn battery(&mut self) -> Result<Battery> {
        let r = self.call(feature::UNIFIED_BATTERY, 1, &[])?;
        Ok(parse_battery(&r))
    }

    pub fn dpi(&mut self) -> Result<DpiCaps> {
        let list = self.call(feature::ADJUSTABLE_DPI, 1, &[0])?;
        let cur = self.call(feature::ADJUSTABLE_DPI, 2, &[0])?;
        // List starts after the sensor index: u16 values, 0xE000|step marks
        // a range between its neighbours, 0 ends the list.
        let mut values = Vec::new();
        let mut step = 0u16;
        for pair in list[1..].chunks_exact(2) {
            let v = u16::from_be_bytes([pair[0], pair[1]]);
            if v == 0 {
                break;
            }
            if v >= 0xE000 {
                step = v & 0x1FFF;
            } else {
                values.push(v);
            }
        }
        let min = values.iter().copied().min().unwrap_or(100);
        let max = values.iter().copied().max().unwrap_or(25_600);
        let current = u16::from_be_bytes([cur[1], cur[2]]);
        let default = u16::from_be_bytes([cur[3], cur[4]]);
        Ok(DpiCaps { min, max, step: step.max(1), current, default: if default == 0 { current } else { default } })
    }

    pub fn set_dpi(&mut self, dpi: u16) -> Result<()> {
        let [hi, lo] = dpi.to_be_bytes();
        self.call(feature::ADJUSTABLE_DPI, 3, &[0, hi, lo])?;
        Ok(())
    }

    pub fn report_rate(&mut self) -> Result<ReportRate> {
        let mask = self.call(feature::REPORT_RATE, 0, &[])?[0];
        let cur = self.call(feature::REPORT_RATE, 1, &[])?[0];
        let supported_ms = (0..8u8).filter(|b| mask & (1 << b) != 0).map(|b| b + 1).collect();
        Ok(ReportRate { current_ms: cur, supported_ms })
    }

    pub fn set_report_rate(&mut self, ms: u8) -> Result<()> {
        self.call(feature::REPORT_RATE, 2, &[ms])?;
        Ok(())
    }

    pub fn onboard(&mut self) -> Result<Onboard> {
        let desc = self.call(feature::ONBOARD_PROFILES, 0, &[])?;
        let mode = self.call(feature::ONBOARD_PROFILES, 2, &[])?[0];
        let cur = self.call(feature::ONBOARD_PROFILES, 4, &[])?;
        Ok(Onboard {
            mode: if mode == 2 { OnboardMode::Host } else { OnboardMode::Onboard },
            profile_count: desc[3],
            current_profile: u16::from_be_bytes([cur[0], cur[1]]),
            button_count: desc[5],
            sector_size: u16::from_be_bytes([desc[7], desc[8]]),
        })
    }

    pub fn set_onboard_mode(&mut self, mode: OnboardMode) -> Result<()> {
        let v = match mode {
            OnboardMode::Onboard => 1,
            OnboardMode::Host => 2,
        };
        self.call(feature::ONBOARD_PROFILES, 1, &[v])?;
        Ok(())
    }

    pub fn set_current_profile(&mut self, profile: u16) -> Result<()> {
        let [hi, lo] = profile.to_be_bytes();
        self.call(feature::ONBOARD_PROFILES, 3, &[hi, lo])?;
        Ok(())
    }

    /// Reads one onboard-memory sector, 16 bytes at a time. The last chunk
    /// is read from `size - 16`, since reads may not run past the end.
    pub fn read_sector(&mut self, sector: u16, size: u16) -> Result<Vec<u8>> {
        let size = size as usize;
        let mut out = vec![0u8; size];
        let mut off = 0;
        while off < size {
            let o = off.min(size.saturating_sub(16));
            let [sh, sl] = sector.to_be_bytes();
            let [oh, ol] = (o as u16).to_be_bytes();
            let chunk = self.call(feature::ONBOARD_PROFILES, 5, &[sh, sl, oh, ol])?;
            let n = 16.min(size - o);
            out[o..o + n].copy_from_slice(&chunk[..n]);
            off += 16;
        }
        Ok(out)
    }

    /// Writes a whole sector (the caller seals it with its CRC first).
    pub fn write_sector(&mut self, sector: u16, data: &[u8]) -> Result<()> {
        let [sh, sl] = sector.to_be_bytes();
        let [lh, ll] = (data.len() as u16).to_be_bytes();
        self.call(feature::ONBOARD_PROFILES, 6, &[sh, sl, 0, 0, lh, ll])?;
        for chunk in data.chunks(16) {
            self.call(feature::ONBOARD_PROFILES, 7, chunk)?;
        }
        self.call(feature::ONBOARD_PROFILES, 8, &[])?;
        Ok(())
    }
}

pub fn parse_battery(r: &[u8]) -> Battery {
    let charging = match r[2] {
        0 => Charging::Discharging,
        1 | 2 => Charging::Charging,
        3 => Charging::Full,
        _ => Charging::Error,
    };
    let percent = if r[0] > 0 {
        r[0].min(100)
    } else {
        // Fall back to the coarse level bits.
        match r[1] {
            l if l & 8 != 0 => 100,
            l if l & 4 != 0 => 60,
            l if l & 2 != 0 => 20,
            _ => 5,
        }
    };
    Battery { percent, charging }
}

/// A Logitech hidraw node that might speak HID++.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub path: PathBuf,
    pub name: String,
    pub product_id: u16,
}

/// Logitech hidraw nodes that are devices, not receivers.
pub fn candidates() -> Vec<Candidate> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/hidraw") else { return out };
    for entry in entries.flatten() {
        let uevent = fs::read_to_string(entry.path().join("device/uevent")).unwrap_or_default();
        let mut id = "";
        let mut name = "";
        for line in uevent.lines() {
            if let Some(v) = line.strip_prefix("HID_ID=") {
                id = v;
            } else if let Some(v) = line.strip_prefix("HID_NAME=") {
                name = v;
            }
        }
        // HID_ID=BBBB:VVVVVVVV:PPPPPPPP
        let mut parts = id.split(':');
        let (_bus, vendor, product) = (parts.next(), parts.next(), parts.next());
        let vendor = vendor.and_then(|v| u32::from_str_radix(v, 16).ok());
        let product = product.and_then(|v| u32::from_str_radix(v, 16).ok()).unwrap_or(0) as u16;
        if vendor != Some(0x046D) || name.to_ascii_lowercase().contains("receiver") {
            continue;
        }
        out.push(Candidate { path: PathBuf::from("/dev").join(entry.file_name()), name: name.to_string(), product_id: product });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}
