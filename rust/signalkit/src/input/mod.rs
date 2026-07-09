//! Touch input: raw decoding, device discovery, and coordinate calibration.
//!
//! Hit testing lives in [`crate::widget::hit_test`]; this module turns the
//! kernel's evdev stream into screen-space [`TouchEvent`]s.

pub mod evdev;

pub use evdev::{TouchDecoder, TouchEvent, TouchKind};

use crate::geometry::{Point, Size};

#[cfg(feature = "fbink")]
use std::fs::File;
#[cfg(feature = "fbink")]
use std::io::{self, Read};
#[cfg(feature = "fbink")]
use std::os::fd::{AsRawFd, RawFd};
#[cfg(feature = "fbink")]
use std::path::Path;

/// `EVIOCGRAB` — exclusively seize an input device so no other process (Kindle
/// framework / X) receives its events. Without this, taps fall through to the
/// home screen underneath our framebuffer drawing.
///
/// C macro: `#define EVIOCGRAB _IOW('E', 0x90, int)` → `0x40044590`.
#[cfg(feature = "fbink")]
const EVIOCGRAB: usize = 0x4004_4590;

/// `EVIOCGABS(abs)` — `_IOR('E', 0x40 + abs, struct input_absinfo)`.
/// `input_absinfo` is 6 × i32 = 24 bytes.
#[cfg(feature = "fbink")]
fn eviocgabs(axis: u8) -> usize {
    (2 << 30) | (24 << 16) | ((b'E' as usize) << 8) | (0x40 + axis as usize)
}

#[cfg(feature = "fbink")]
const ABS_MT_POSITION_X: u8 = 0x35;
#[cfg(feature = "fbink")]
const ABS_MT_POSITION_Y: u8 = 0x36;

/// Kernel `struct input_absinfo` returned by `EVIOCGABS`.
#[cfg(feature = "fbink")]
#[repr(C)]
#[derive(Default)]
struct InputAbsinfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

#[cfg(feature = "fbink")]
extern "C" {
    fn ioctl(fd: i32, request: usize, ...) -> i32;
}

/// An open touchscreen fd held under an exclusive `EVIOCGRAB`.
///
/// The grab keeps the Kindle UI from also seeing our taps. Dropping releases
/// the grab so the framework can take input again.
#[cfg(feature = "fbink")]
pub struct GrabbedTouch {
    file: File,
    fd: RawFd,
}

#[cfg(feature = "fbink")]
impl GrabbedTouch {
    /// Opens `path` and exclusively grabs it. Fails if the device is missing or
    /// another process already holds the grab.
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let fd = file.as_raw_fd();
        let ret = unsafe { ioctl(fd, EVIOCGRAB, 1_i32) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(GrabbedTouch { file, fd })
    }

    /// Queries the digitizer's ABS_MT axis maxima via `EVIOCGABS`, falling back
    /// to the screen size when the ioctl fails. Used to build a
    /// [`Calibration`] that scales raw touch coords into screen pixels.
    pub fn axis_max(&self) -> (i32, i32) {
        let max_x = Self::query_axis_max(self.fd, ABS_MT_POSITION_X).unwrap_or(0);
        let max_y = Self::query_axis_max(self.fd, ABS_MT_POSITION_Y).unwrap_or(0);
        (max_x, max_y)
    }

    fn query_axis_max(fd: RawFd, axis: u8) -> Option<i32> {
        let mut info = InputAbsinfo::default();
        let ret = unsafe { ioctl(fd, eviocgabs(axis), &mut info as *mut InputAbsinfo) };
        if ret < 0 || info.maximum <= 0 {
            None
        } else {
            Some(info.maximum)
        }
    }

    /// Blocks until a raw `input_event` record is available, then reads it into
    /// `buf` (must be [`evdev::EVENT_SIZE`]).
    pub fn read_record(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.file.read_exact(buf)
    }
}

#[cfg(feature = "fbink")]
impl Drop for GrabbedTouch {
    fn drop(&mut self) {
        // Release before the fd closes so the framework can reclaim input.
        unsafe {
            ioctl(self.fd, EVIOCGRAB, 0_i32);
        }
    }
}

/// Maps raw device coordinates to screen pixels. Kindle panels vary in touch
/// resolution, axis order, and inversion between models, so this is
/// configurable (and overridable via env — see [`Calibration::from_env`]).
#[derive(Debug, Clone, Copy)]
pub struct Calibration {
    /// Raw coordinate range reported by the digitizer.
    pub raw: Size,
    /// Target screen size in pixels.
    pub screen: Size,
    /// Swap X and Y (portrait/landscape digitizer mismatch).
    pub swap_xy: bool,
    /// Invert the (post-swap) X axis.
    pub invert_x: bool,
    /// Invert the (post-swap) Y axis.
    pub invert_y: bool,
}

impl Calibration {
    /// Identity mapping: raw coordinates already match screen pixels.
    pub fn identity(screen: Size) -> Self {
        Calibration {
            raw: screen,
            screen,
            swap_xy: false,
            invert_x: false,
            invert_y: false,
        }
    }

    /// Builds a calibration from the digitizer's reported axis maxima, then
    /// applies the same env overrides as [`from_env`](Self::from_env).
    ///
    /// When `raw_max` is `(0, 0)` (ioctl failed), falls back to identity.
    pub fn from_device(screen: Size, raw_max: (i32, i32)) -> Self {
        let mut c = if raw_max.0 > 0 && raw_max.1 > 0 {
            Calibration {
                raw: Size::new(raw_max.0, raw_max.1),
                screen,
                swap_xy: false,
                invert_x: false,
                invert_y: false,
            }
        } else {
            Calibration::identity(screen)
        };
        c.apply_env_overrides();
        c
    }

    /// Reads overrides from the environment, falling back to identity:
    /// `SIGNALKIT_TOUCH_SWAP=1`, `SIGNALKIT_TOUCH_INVERT_X=1`,
    /// `SIGNALKIT_TOUCH_INVERT_Y=1`, `SIGNALKIT_TOUCH_RAW=WxH`.
    pub fn from_env(screen: Size) -> Self {
        let mut c = Calibration::identity(screen);
        c.apply_env_overrides();
        c
    }

    fn apply_env_overrides(&mut self) {
        let flag = |k: &str| std::env::var(k).map(|v| v == "1").unwrap_or(false);
        self.swap_xy = flag("SIGNALKIT_TOUCH_SWAP");
        self.invert_x = flag("SIGNALKIT_TOUCH_INVERT_X");
        self.invert_y = flag("SIGNALKIT_TOUCH_INVERT_Y");
        if let Ok(raw) = std::env::var("SIGNALKIT_TOUCH_RAW") {
            if let Some((w, h)) = raw.split_once('x') {
                if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
                    self.raw = Size::new(w, h);
                }
            }
        }
    }

    /// Applies the calibration to a raw touch point.
    pub fn map(&self, raw_x: i32, raw_y: i32) -> Point {
        let (mut rx, mut ry) = if self.swap_xy {
            (raw_y, raw_x)
        } else {
            (raw_x, raw_y)
        };
        // Post-swap raw ranges.
        let (rw, rh) = if self.swap_xy {
            (self.raw.h, self.raw.w)
        } else {
            (self.raw.w, self.raw.h)
        };
        if self.invert_x {
            rx = rw.saturating_sub(rx);
        }
        if self.invert_y {
            ry = rh.saturating_sub(ry);
        }
        let sx = if rw > 0 { rx * self.screen.w / rw } else { rx };
        let sy = if rh > 0 { ry * self.screen.h / rh } else { ry };
        Point::new(
            sx.clamp(0, self.screen.w - 1),
            sy.clamp(0, self.screen.h - 1),
        )
    }
}

/// Finds the touchscreen device node by scanning `/proc/bus/input/devices`.
///
/// Honors the `SIGNALKIT_TOUCH_DEV` override. Otherwise picks the first device
/// whose event-type bitmask advertises `EV_ABS` (absolute axes) and exposes an
/// `eventN` handler — the capability-based discovery the plan calls for, no
/// ioctl required.
#[cfg(feature = "fbink")]
pub fn find_touch_device() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;

    if let Ok(dev) = std::env::var("SIGNALKIT_TOUCH_DEV") {
        return Some(PathBuf::from(dev));
    }
    let text = std::fs::read_to_string("/proc/bus/input/devices").ok()?;
    let node = parse_touch_device(&text)?;
    Some(PathBuf::from(format!("/dev/input/{node}")))
}

/// Pure parser for `/proc/bus/input/devices` content; returns the `eventN` node
/// of the first absolute-axis device. Separated out so it is host-testable
/// (used at runtime only by [`find_touch_device`], behind the `fbink` feature).
#[cfg_attr(not(feature = "fbink"), allow(dead_code))]
pub(crate) fn parse_touch_device(text: &str) -> Option<String> {
    for block in text.split("\n\n") {
        let mut handler: Option<String> = None;
        let mut has_abs = false;
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("H: Handlers=") {
                for tok in rest.split_whitespace() {
                    if tok.starts_with("event") {
                        handler = Some(tok.to_string());
                    }
                }
            } else if let Some(rest) = line.strip_prefix("B: EV=") {
                // Hex bitmask of supported event types; EV_ABS == bit 3.
                if let Ok(bits) = u64::from_str_radix(rest.trim(), 16) {
                    has_abs = bits & (1 << 3) != 0;
                }
            }
        }
        if has_abs {
            if let Some(h) = handler {
                return Some(h);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibration_maps_and_inverts() {
        let c = Calibration {
            raw: Size::new(1000, 2000),
            screen: Size::new(100, 200),
            swap_xy: false,
            invert_x: false,
            invert_y: true,
        };
        // (500, 0) raw -> x=50, y inverted from top so 2000->0 => screen 199.
        let p = c.map(500, 0);
        assert_eq!(p.x, 50);
        assert_eq!(p.y, 199);
    }

    #[test]
    fn parse_picks_abs_device_event_node() {
        let text = "\
I: Bus=0019 Vendor=0000 Product=0000
N: Name=\"gpio-keys\"
H: Handlers=kbd event0
B: EV=3

I: Bus=0018 Vendor=0000 Product=0000
N: Name=\"cyttsp5_mt\"
H: Handlers=event1
B: EV=b
";
        // EV=3 -> bits 0,1 (SYN,KEY) no ABS; EV=b -> 0b1011 has bit 3 (ABS).
        assert_eq!(parse_touch_device(text), Some("event1".to_string()));
    }
}
