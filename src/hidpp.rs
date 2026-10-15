//! Minimal HID++ 2.0 transport over a Linux hidraw node.
//!
//! Requests are sent as long (20-byte) reports. The device index byte is
//! 0xFF; when the node belongs to a device paired with a receiver, the
//! kernel's hid-logitech-dj driver rewrites it to the right slot.

use std::collections::VecDeque;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::time::{Duration, Instant};

const REPORT_SHORT: u8 = 0x10;
const REPORT_LONG: u8 = 0x11;
const LONG_LEN: usize = 20;
const DEVICE_INDEX: u8 = 0xFF;
/// Software id in the low nibble of the function byte; tells our replies
/// apart from Solaar's or the kernel's.
const SW_ID: u8 = 0x0A;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Timeout,
    /// HID++ 2.0 error code from the device.
    Device(u8),
    /// HID++ 1.0 error (usually: device asleep or out of range).
    Unreachable,
    Unsupported(u16),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Timeout => write!(f, "the mouse did not answer"),
            Error::Device(code) => write!(f, "{}", device_error_name(*code)),
            Error::Unreachable => write!(f, "the mouse is asleep or out of range"),
            Error::Unsupported(id) => write!(f, "feature 0x{id:04x} is not supported by this mouse"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

fn device_error_name(code: u8) -> &'static str {
    match code {
        1 => "unknown error",
        2 => "invalid argument",
        3 => "value out of range",
        4 => "hardware error",
        5 => "internal error",
        6 => "invalid feature index",
        7 => "invalid function",
        8 => "device busy",
        9 => "not allowed in the current mode",
        _ => "device error",
    }
}

/// An unsolicited report (notification) from the device.
#[derive(Debug, Clone, Copy)]
pub struct Notification {
    pub feature_index: u8,
    pub data: [u8; 16],
}

pub struct Transport {
    file: File,
    pending: VecDeque<Notification>,
}

impl Transport {
    pub fn open(path: &Path) -> Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        Ok(Self { file, pending: VecDeque::new() })
    }

    /// Sends a request and waits for its reply, retrying once on timeout
    /// (a sleeping wireless mouse often drops the first packet).
    pub fn request(&mut self, feature_index: u8, function: u8, params: &[u8]) -> Result<[u8; 16]> {
        match self.request_once(feature_index, function, params, Duration::from_millis(800)) {
            Err(Error::Timeout) => self.request_once(feature_index, function, params, Duration::from_millis(1500)),
            other => other,
        }
    }

    fn request_once(&mut self, feature_index: u8, function: u8, params: &[u8], timeout: Duration) -> Result<[u8; 16]> {
        let func_sw = (function << 4) | SW_ID;
        let mut msg = [0u8; LONG_LEN];
        msg[0] = REPORT_LONG;
        msg[1] = DEVICE_INDEX;
        msg[2] = feature_index;
        msg[3] = func_sw;
        let n = params.len().min(16);
        msg[4..4 + n].copy_from_slice(&params[..n]);
        self.file.write_all(&msg)?;

        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(Error::Timeout);
            }
            let Some(buf) = self.read_report(left)? else { return Err(Error::Timeout) };
            if buf[0] != REPORT_SHORT && buf[0] != REPORT_LONG {
                continue;
            }
            // HID++ 1.0 error: 10 ix 8F <feature> <func_sw> <code>
            if buf[2] == 0x8F && buf[3] == feature_index && buf[4] == func_sw {
                return Err(Error::Unreachable);
            }
            // HID++ 2.0 error: 11 ix FF <feature> <func_sw> <code>
            if buf[2] == 0xFF && buf[3] == feature_index && buf[4] == func_sw {
                return Err(Error::Device(buf[5]));
            }
            if buf[2] == feature_index && buf[3] == func_sw {
                let mut data = [0u8; 16];
                data.copy_from_slice(&buf[4..20]);
                return Ok(data);
            }
            // Anything with software id 0 is a notification worth keeping.
            if buf[3] & 0x0F == 0 && buf[2] != 0xFF && buf[2] != 0x8F {
                self.pending.push_back(to_notification(&buf));
            }
        }
    }

    /// Returns the next notification, waiting up to `timeout`.
    pub fn next_notification(&mut self, timeout: Duration) -> Result<Option<Notification>> {
        if let Some(n) = self.pending.pop_front() {
            return Ok(Some(n));
        }
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let Some(buf) = self.read_report(left)? else { return Ok(None) };
            if (buf[0] == REPORT_SHORT || buf[0] == REPORT_LONG) && buf[3] & 0x0F == 0 && buf[2] != 0xFF && buf[2] != 0x8F {
                return Ok(Some(to_notification(&buf)));
            }
            if left.is_zero() {
                return Ok(None);
            }
        }
    }

    /// Reads one report, normalised to 20 bytes. `None` on timeout.
    fn read_report(&mut self, timeout: Duration) -> Result<Option<[u8; LONG_LEN]>> {
        let mut pfd = libc::pollfd { fd: self.file.as_raw_fd(), events: libc::POLLIN, revents: 0 };
        let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
        // SAFETY: pfd points to one valid pollfd for the duration of the call.
        let rc = unsafe { libc::poll(&mut pfd, 1, ms) };
        if rc < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                return Ok(None);
            }
            return Err(err.into());
        }
        if rc == 0 {
            return Ok(None);
        }
        if pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            return Err(Error::Io(io::Error::new(io::ErrorKind::NotConnected, "device removed")));
        }
        let mut raw = [0u8; 64];
        let n = self.file.read(&mut raw)?;
        let mut buf = [0u8; LONG_LEN];
        let n = n.min(LONG_LEN);
        buf[..n].copy_from_slice(&raw[..n]);
        Ok(Some(buf))
    }
}

fn to_notification(buf: &[u8; LONG_LEN]) -> Notification {
    let mut data = [0u8; 16];
    data.copy_from_slice(&buf[4..20]);
    Notification { feature_index: buf[2], data }
}
