//! Rendering abstraction.
//!
//! SignalKit delegates all drawing to UIKit/AppKit. The Kindle target has no
//! such retained view server, so this module defines the seam the port adds: a
//! [`Renderer`] receives a batch of [`DrawCmd`]s and refreshes regions of the
//! e-ink panel. The FBInk backend ([`fbink`]) implements it on-device; the
//! [`mock`] backend records commands for host tests.

use crate::geometry::Rect;

pub mod mock;

#[cfg(feature = "fbink")]
pub mod fbink;

/// A 4-bit grayscale level, matching e-ink's native depth. `0` is black, `15`
/// is white. FBInk also thinks in 8-bit gray; we scale by 17 (`0x11`) at the
/// backend so `WHITE.to_gray8() == 255`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub u8);

impl Color {
    pub const BLACK: Color = Color(0);
    pub const WHITE: Color = Color(15);
    /// Mid gray, useful for dividers and disabled text.
    pub const GRAY: Color = Color(8);

    /// Clamps to the 0..=15 range.
    pub fn level(self) -> u8 {
        self.0.min(15)
    }

    /// Expands to the 0..=255 gray value FBInk expects.
    pub fn to_gray8(self) -> u8 {
        self.level() * 0x11
    }
}

/// How aggressively to refresh a region on the e-ink panel.
///
/// Partial (fast) refreshes leave faint ghosting behind; a full refresh flashes
/// the panel to clear it. The app escalates to [`RefreshMode::Full`]
/// periodically — see [`crate::app`]'s refresh policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshMode {
    /// Fast partial update (FBInk DU/A2-style waveform). May ghost.
    Partial,
    /// Full flashing update (FBInk GC16) that clears ghosting.
    Full,
}

/// A single primitive draw operation, in absolute screen pixels. Widgets emit
/// these during paint; the [`Renderer`] translates them to the backend.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawCmd {
    /// Fill `rect` with a solid gray level (used for backgrounds and clears).
    FillRect { rect: Rect, color: Color },
    /// Draw `text` with its top-left at (`x`, `y`). `size` is a font size
    /// multiplier (1 = base font). `inverse` swaps fg/bg (used for pressed
    /// buttons — cheap on e-ink).
    Text {
        x: i32,
        y: i32,
        text: String,
        size: u8,
        fg: Color,
        bg: Color,
        inverse: bool,
    },
}

/// Something that can draw [`DrawCmd`]s and refresh e-ink regions.
///
/// The contract, per frame: the app calls [`submit`](Renderer::submit) with the
/// batch of commands for all damaged widgets (each drawn with refresh
/// suppressed), then [`refresh`](Renderer::refresh) once for the union of the
/// damaged region. This keeps e-ink refreshes to one per frame regardless of
/// how many widgets changed.
pub trait Renderer {
    /// The usable panel size in pixels. Queried once at startup.
    fn screen_size(&mut self) -> crate::geometry::Size;

    /// Draws a batch of commands without refreshing the panel.
    fn submit(&mut self, cmds: &[DrawCmd]) -> std::io::Result<()>;

    /// Refreshes `region` with the given waveform mode.
    fn refresh(&mut self, region: Rect, mode: RefreshMode) -> std::io::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gray_scaling_spans_full_range() {
        assert_eq!(Color::BLACK.to_gray8(), 0);
        assert_eq!(Color::WHITE.to_gray8(), 255);
        assert_eq!(Color::GRAY.to_gray8(), 136);
    }
}
