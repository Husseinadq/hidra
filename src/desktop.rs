//! Pointer settings owned by the desktop (GNOME), read and written with
//! `gsettings`. They apply to every mouse, not just this one.

use std::process::Command;

const SCHEMA: &str = "org.gnome.desktop.peripherals.mouse";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accel {
    Default,
    Flat,
    Adaptive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Desktop {
    pub accel: Accel,
    /// -1.0 … 1.0
    pub speed: f64,
    pub natural_scroll: bool,
}

fn get(key: &str) -> Option<String> {
    let out = Command::new("gsettings").args(["get", SCHEMA, key]).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().trim_matches('\'').to_string())
}

fn set(key: &str, value: &str) -> Result<(), String> {
    let out = Command::new("gsettings").args(["set", SCHEMA, key, value]).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

impl Desktop {
    /// `None` when not on GNOME or `gsettings` is missing.
    pub fn read() -> Option<Self> {
        let accel = match get("accel-profile")?.as_str() {
            "flat" => Accel::Flat,
            "adaptive" => Accel::Adaptive,
            _ => Accel::Default,
        };
        let speed = get("speed")?.parse().unwrap_or(0.0);
        let natural_scroll = get("natural-scroll")? == "true";
        Some(Self { accel, speed, natural_scroll })
    }

    pub fn set_accel(a: Accel) -> Result<(), String> {
        set(
            "accel-profile",
            match a {
                Accel::Default => "default",
                Accel::Flat => "flat",
                Accel::Adaptive => "adaptive",
            },
        )
    }

    pub fn set_speed(v: f64) -> Result<(), String> {
        set("speed", &format!("{:.2}", v.clamp(-1.0, 1.0)))
    }

    pub fn set_natural_scroll(on: bool) -> Result<(), String> {
        set("natural-scroll", if on { "true" } else { "false" })
    }
}
