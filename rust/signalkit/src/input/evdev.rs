//! Hand-rolled Linux evdev decoding.
//!
//! Kindle touchscreens report through `/dev/input/event*` as a stream of fixed
//! size `input_event` records. We parse them directly rather than pull in the
//! `evdev` crate — a read-only single-touch decoder is ~a screenful of code and
//! keeps the cross-compile dependency count at zero. Both multitouch protocol B
//! (`ABS_MT_TRACKING_ID`) and the older protocol A / `BTN_TOUCH` are handled.

use std::mem::size_of;
use std::os::raw::c_long;

// event .type values
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

// codes
const SYN_REPORT: u16 = 0x00;
const BTN_TOUCH: u16 = 0x14a;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;
const ABS_MT_TRACKING_ID: u16 = 0x39;

/// Byte size of one `struct input_event` for the target's C layout:
/// `struct timeval { long tv_sec; long tv_usec; }` (2 longs) + `u16 type` +
/// `u16 code` + `i32 value`. On ARM32 this is 16 bytes; on 64-bit hosts, 24.
pub const EVENT_SIZE: usize = 2 * size_of::<c_long>() + 2 + 2 + 4;

const TYPE_OFFSET: usize = 2 * size_of::<c_long>();

/// A decoded touch gesture point, in raw device coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TouchEvent {
    pub kind: TouchKind,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchKind {
    Down,
    Move,
    Up,
}

/// Accumulates raw `input_event` records into touch gestures, emitting one
/// [`TouchEvent`] per completed `SYN_REPORT` frame that changed state.
#[derive(Debug, Default)]
pub struct TouchDecoder {
    x: i32,
    y: i32,
    down: bool,
    // Per-frame pending changes, applied on SYN_REPORT.
    pending_pos: bool,
    pending_down: Option<bool>,
}

impl TouchDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one raw event record (`EVENT_SIZE` bytes). Returns a gesture event
    /// when a frame completes with a meaningful change.
    pub fn feed(&mut self, record: &[u8]) -> Option<TouchEvent> {
        if record.len() < EVENT_SIZE {
            return None;
        }
        let etype = u16::from_le_bytes([record[TYPE_OFFSET], record[TYPE_OFFSET + 1]]);
        let code = u16::from_le_bytes([record[TYPE_OFFSET + 2], record[TYPE_OFFSET + 3]]);
        let value = i32::from_le_bytes([
            record[TYPE_OFFSET + 4],
            record[TYPE_OFFSET + 5],
            record[TYPE_OFFSET + 6],
            record[TYPE_OFFSET + 7],
        ]);

        match etype {
            EV_ABS => match code {
                ABS_X | ABS_MT_POSITION_X => {
                    self.x = value;
                    self.pending_pos = true;
                }
                ABS_Y | ABS_MT_POSITION_Y => {
                    self.y = value;
                    self.pending_pos = true;
                }
                ABS_MT_TRACKING_ID => {
                    // -1 = finger lifted (protocol B); >=0 = finger down.
                    self.pending_down = Some(value >= 0);
                }
                _ => {}
            },
            EV_KEY if code == BTN_TOUCH => {
                self.pending_down = Some(value != 0);
            }
            EV_SYN if code == SYN_REPORT => {
                return self.flush_frame();
            }
            _ => {}
        }
        None
    }

    fn flush_frame(&mut self) -> Option<TouchEvent> {
        let pos_changed = self.pending_pos;
        let down_change = self.pending_down.take();
        self.pending_pos = false;

        let event = match down_change {
            Some(true) if !self.down => {
                self.down = true;
                Some(TouchKind::Down)
            }
            Some(false) if self.down => {
                self.down = false;
                Some(TouchKind::Up)
            }
            _ if self.down && pos_changed => Some(TouchKind::Move),
            _ => None,
        };

        event.map(|kind| TouchEvent {
            kind,
            x: self.x,
            y: self.y,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a raw event record for the host's layout.
    fn ev(etype: u16, code: u16, value: i32) -> Vec<u8> {
        let mut buf = vec![0u8; EVENT_SIZE];
        buf[TYPE_OFFSET..TYPE_OFFSET + 2].copy_from_slice(&etype.to_le_bytes());
        buf[TYPE_OFFSET + 2..TYPE_OFFSET + 4].copy_from_slice(&code.to_le_bytes());
        buf[TYPE_OFFSET + 4..TYPE_OFFSET + 8].copy_from_slice(&value.to_le_bytes());
        buf
    }

    #[test]
    fn protocol_b_tap_yields_down_then_up() {
        let mut d = TouchDecoder::new();
        // Finger down at (100, 200).
        assert_eq!(d.feed(&ev(EV_ABS, ABS_MT_TRACKING_ID, 5)), None);
        assert_eq!(d.feed(&ev(EV_ABS, ABS_MT_POSITION_X, 100)), None);
        assert_eq!(d.feed(&ev(EV_ABS, ABS_MT_POSITION_Y, 200)), None);
        assert_eq!(
            d.feed(&ev(EV_SYN, SYN_REPORT, 0)),
            Some(TouchEvent {
                kind: TouchKind::Down,
                x: 100,
                y: 200
            })
        );
        // Finger up.
        assert_eq!(d.feed(&ev(EV_ABS, ABS_MT_TRACKING_ID, -1)), None);
        assert_eq!(
            d.feed(&ev(EV_SYN, SYN_REPORT, 0)),
            Some(TouchEvent {
                kind: TouchKind::Up,
                x: 100,
                y: 200
            })
        );
    }

    #[test]
    fn protocol_a_btn_touch_and_move() {
        let mut d = TouchDecoder::new();
        d.feed(&ev(EV_KEY, BTN_TOUCH, 1));
        d.feed(&ev(EV_ABS, ABS_X, 10));
        d.feed(&ev(EV_ABS, ABS_Y, 20));
        assert_eq!(d.feed(&ev(EV_SYN, SYN_REPORT, 0)).unwrap().kind, TouchKind::Down);

        d.feed(&ev(EV_ABS, ABS_X, 30));
        let mv = d.feed(&ev(EV_SYN, SYN_REPORT, 0)).unwrap();
        assert_eq!(mv.kind, TouchKind::Move);
        assert_eq!((mv.x, mv.y), (30, 20));
    }

    #[test]
    fn empty_frame_emits_nothing() {
        let mut d = TouchDecoder::new();
        assert_eq!(d.feed(&ev(EV_SYN, SYN_REPORT, 0)), None);
    }
}
