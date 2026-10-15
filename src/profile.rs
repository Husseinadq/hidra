//! Button bindings inside an HID++ onboard profile (feature 0x8100).
//!
//! A profile sector holds a table of 4-byte bindings starting at byte 32:
//! `80 01 <mask>` sends mouse buttons, `80 02 <mods> <key>` a keyboard key,
//! `80 03 <usage>` a consumer (media) key, `90 <fn> 00 00` a mouse function
//! such as DPI shift, and `FF FF FF FF` means unassigned.

pub type Raw = [u8; 4];

pub const BUTTONS_OFFSET: usize = 32;
pub const BUTTON_COUNT: usize = 16;

const MOD_CTRL: u8 = 0x01;
const MOD_SHIFT: u8 = 0x02;
const MOD_ALT: u8 = 0x04;
const MOD_SUPER: u8 = 0x08;

const KEY_TAB: u8 = 0x2B;
const KEY_PRINT: u8 = 0x46;
const KEY_PAGE_UP: u8 = 0x4B;
const KEY_PAGE_DOWN: u8 = 0x4E;

fn mouse(mask: u16) -> Raw {
    let [hi, lo] = mask.to_be_bytes();
    [0x80, 0x01, hi, lo]
}
fn key(mods: u8, key: u8) -> Raw {
    [0x80, 0x02, mods, key]
}
fn consumer(usage: u16) -> Raw {
    let [hi, lo] = usage.to_be_bytes();
    [0x80, 0x03, hi, lo]
}
fn function(f: u8) -> Raw {
    [0x90, f, 0x00, 0x00]
}

/// The binding the mouse should store for an action.
pub fn binding_for(action: &str, shortcut: Option<&str>) -> Result<Raw, String> {
    Ok(match action {
        "lclick" => mouse(0x0001),
        "rclick" => mouse(0x0002),
        "mclick" => mouse(0x0004),
        "back" => mouse(0x0008),
        "forward" => mouse(0x0010),
        "sleft" => function(0x01),
        "sright" => function(0x02),
        "dpiup" => function(0x03),
        "dpidown" => function(0x04),
        "dpicycle" => function(0x05),
        "dpishift" => function(0x07),
        "cycleprofile" => function(0x0A),
        "none" => function(0x00),
        "nexttab" => key(MOD_CTRL, KEY_TAB),
        "prevtab" => key(MOD_CTRL | MOD_SHIFT, KEY_TAB),
        "workspaces" => key(MOD_SUPER, 0),
        "wsnext" => key(MOD_SUPER, KEY_PAGE_DOWN),
        "wsprev" => key(MOD_SUPER, KEY_PAGE_UP),
        "screenshot" => key(0, KEY_PRINT),
        "play" => consumer(0x00CD),
        "volup" => consumer(0x00E9),
        "voldown" => consumer(0x00EA),
        "mute" => consumer(0x00E2),
        "key" => {
            let (mods, k) = parse_shortcut(shortcut.unwrap_or(""))?;
            key(mods, k)
        }
        other => return Err(format!("“{}” can't be stored on the mouse", crate::catalog::action_label(other))),
    })
}

/// The action a stored binding corresponds to, plus shortcut text for keys.
pub fn action_for(raw: Raw) -> (&'static str, Option<String>) {
    match raw {
        [0xFF, 0xFF, 0xFF, 0xFF] => ("none", None),
        [0x80, 0x01, hi, lo] => match u16::from_be_bytes([hi, lo]) {
            0x0001 => ("lclick", None),
            0x0002 => ("rclick", None),
            0x0004 => ("mclick", None),
            0x0008 => ("back", None),
            0x0010 => ("forward", None),
            _ => ("other", None),
        },
        [0x80, 0x02, mods, k] => match (mods, k) {
            (MOD_CTRL, KEY_TAB) => ("nexttab", None),
            (m, KEY_TAB) if m == MOD_CTRL | MOD_SHIFT => ("prevtab", None),
            (MOD_SUPER, 0) => ("workspaces", None),
            (MOD_SUPER, KEY_PAGE_DOWN) => ("wsnext", None),
            (MOD_SUPER, KEY_PAGE_UP) => ("wsprev", None),
            (0, KEY_PRINT) => ("screenshot", None),
            _ => ("key", Some(format_shortcut(mods, k))),
        },
        [0x80, 0x03, hi, lo] => match u16::from_be_bytes([hi, lo]) {
            0x00CD => ("play", None),
            0x00E9 => ("volup", None),
            0x00EA => ("voldown", None),
            0x00E2 => ("mute", None),
            _ => ("other", None),
        },
        [0x90, f, _, _] => match f {
            0x00 => ("none", None),
            0x01 => ("sleft", None),
            0x02 => ("sright", None),
            0x03 => ("dpiup", None),
            0x04 => ("dpidown", None),
            0x05 => ("dpicycle", None),
            0x07 => ("dpishift", None),
            0x08 | 0x0A => ("cycleprofile", None),
            _ => ("other", None),
        },
        _ => ("other", None),
    }
}

const NAMED_KEYS: &[(&str, u8)] = &[
    ("enter", 0x28),
    ("return", 0x28),
    ("esc", 0x29),
    ("escape", 0x29),
    ("backspace", 0x2A),
    ("tab", 0x2B),
    ("space", 0x2C),
    ("minus", 0x2D),
    ("-", 0x2D),
    ("equal", 0x2E),
    ("=", 0x2E),
    ("print", 0x46),
    ("printscreen", 0x46),
    ("insert", 0x49),
    ("home", 0x4A),
    ("pageup", 0x4B),
    ("delete", 0x4C),
    ("del", 0x4C),
    ("end", 0x4D),
    ("pagedown", 0x4E),
    ("right", 0x4F),
    ("left", 0x50),
    ("down", 0x51),
    ("up", 0x52),
];

fn key_code(name: &str) -> Option<u8> {
    let n = name.to_ascii_lowercase();
    let b = n.as_bytes();
    if b.len() == 1 && b[0].is_ascii_lowercase() {
        return Some(0x04 + (b[0] - b'a'));
    }
    if b.len() == 1 && b[0].is_ascii_digit() {
        return Some(if b[0] == b'0' { 0x27 } else { 0x1E + (b[0] - b'1') });
    }
    if let Some(f) = n.strip_prefix('f').and_then(|x| x.parse::<u8>().ok()) {
        if (1..=12).contains(&f) {
            return Some(0x3A + f - 1);
        }
    }
    NAMED_KEYS.iter().find(|(k, _)| *k == n).map(|(_, c)| *c)
}

fn key_name(code: u8) -> String {
    match code {
        0x04..=0x1D => ((b'A' + code - 0x04) as char).to_string(),
        0x1E..=0x26 => ((b'1' + code - 0x1E) as char).to_string(),
        0x27 => "0".into(),
        0x3A..=0x45 => format!("F{}", code - 0x3A + 1),
        _ => NAMED_KEYS
            .iter()
            .find(|(_, c)| *c == code)
            .map(|(k, _)| {
                let mut s = k.to_string();
                s[..1].make_ascii_uppercase();
                s
            })
            .unwrap_or_else(|| format!("Key 0x{code:02X}")),
    }
}

/// Parses text like "Ctrl+Shift+T" into HID modifier bits and a key code.
pub fn parse_shortcut(text: &str) -> Result<(u8, u8), String> {
    let mut mods = 0u8;
    let mut code = None;
    for part in text.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= MOD_CTRL,
            "shift" => mods |= MOD_SHIFT,
            "alt" => mods |= MOD_ALT,
            "super" | "win" | "meta" | "cmd" => mods |= MOD_SUPER,
            _ => {
                if code.is_some() {
                    return Err("The mouse can store one key plus modifiers".into());
                }
                code = Some(key_code(part).ok_or_else(|| format!("Unknown key “{part}”"))?);
            }
        }
    }
    match (mods, code) {
        (0, None) => Err("Type a shortcut, for example Ctrl+Shift+T".into()),
        (m, c) => Ok((m, c.unwrap_or(0))),
    }
}

pub fn format_shortcut(mods: u8, code: u8) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (bit, name) in [(MOD_CTRL, "Ctrl"), (MOD_SHIFT, "Shift"), (MOD_ALT, "Alt"), (MOD_SUPER, "Super")] {
        if mods & bit != 0 {
            parts.push(name.into());
        }
    }
    if code != 0 {
        parts.push(key_name(code));
    }
    parts.join("+")
}

/// CRC-CCITT (0x1021, initial 0xFFFF) used to seal each profile sector.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

pub fn crc_ok(sector: &[u8]) -> bool {
    sector.len() > 2 && crc16(&sector[..sector.len() - 2]).to_be_bytes() == sector[sector.len() - 2..]
}

pub fn seal(sector: &mut [u8]) {
    let n = sector.len();
    let crc = crc16(&sector[..n - 2]).to_be_bytes();
    sector[n - 2..].copy_from_slice(&crc);
}

pub fn buttons(sector: &[u8]) -> Vec<Raw> {
    (0..BUTTON_COUNT)
        .map(|i| {
            let o = BUTTONS_OFFSET + i * 4;
            [sector[o], sector[o + 1], sector[o + 2], sector[o + 3]]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_round_trip() {
        let (m, k) = parse_shortcut("ctrl + shift + t").unwrap();
        assert_eq!((m, k), (MOD_CTRL | MOD_SHIFT, 0x17));
        assert_eq!(format_shortcut(m, k), "Ctrl+Shift+T");
        assert_eq!(parse_shortcut("Super").unwrap(), (MOD_SUPER, 0));
        assert!(parse_shortcut("Ctrl+A+B").is_err());
        assert_eq!(format_shortcut(0, 0x3A + 4), "F5");
    }

    #[test]
    fn every_storable_action_round_trips() {
        for g in crate::catalog::ACTION_GROUPS {
            for (id, _) in g.actions {
                if crate::catalog::needs_software(id) || *id == "key" {
                    continue;
                }
                let raw = binding_for(id, None).unwrap();
                assert_eq!(action_for(raw).0, *id, "{id}");
            }
        }
    }

    #[test]
    fn crc_matches_known_sector() {
        // Profile directory read from a real G502 X PLUS; it stores CRC 0x8232.
        let mut s = vec![0xFFu8; 255];
        s[..20].copy_from_slice(&[0, 1, 1, 0xFF, 0, 2, 1, 0xFF, 0, 3, 0, 0xFF, 0, 4, 0, 0xFF, 0, 5, 0, 0xFF]);
        assert_eq!(crc16(&s[..253]), 0x8232);
        seal(&mut s);
        assert!(crc_ok(&s));
    }
}
