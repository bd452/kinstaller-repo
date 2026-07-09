//! FBInk framebuffer backend (on-device only, `--features fbink`).
//!
//! Translates [`DrawCmd`]s into FBInk calls against the statically-linked
//! library. Text uses FBInk's built-in fixed-cell font (positioned by pixel via
//! `hoffset`/`voffset`), so no FreeType/OpenType path is pulled in. Fills use
//! `fbink_cls` on a rectangle. Every draw call sets `no_refresh`; the panel is
//! updated once per frame by [`FbinkRenderer::refresh`].
//!
//! **Font size:** FBInk only applies `FBInkConfig::fontmult` at `fbink_init`
//! time — setting it on a later `fbink_print` is a no-op. We therefore track the
//! live multiplier and re-init whenever a draw asks for a different size, so
//! layout's 8×16×size cell metrics match what is actually painted.
//!
//! **Pen offsets:** `fbink_print` does not paint exactly at
//! `(hoffset, voffset)`. When the panel width is not an exact multiple of the
//! cell width, FBInk centers the whole column grid by shifting every print
//! right by half the leftover ("dead zone"); similarly it can shift rows down
//! to balance the leftover height, and viewport devices add their own origin.
//! All of these depend on the current `fontmult`, so they change between
//! sizes. `fbink_cls` and `fbink_refresh`, however, use raw framebuffer
//! coordinates. Left uncompensated, glyphs land shifted relative to the frames
//! the layout computed, so damage erases/refreshes miss slivers of old text —
//! stale pixels that never update. After every init we read the offsets back
//! from `fbink_get_state` and subtract them from the print offsets, putting
//! print, cls, and refresh in the same coordinate space.

use std::ffi::CString;
use std::io;

use fbink_sys as sys;

use crate::geometry::{Rect, Size};
use crate::render::{Color, DrawCmd, RefreshMode, Renderer};

/// A renderer backed by FBInk on the Kindle framebuffer.
pub struct FbinkRenderer {
    fbfd: i32,
    screen: Size,
    /// Multiplier last passed to `fbink_init`. Must match the size we draw with.
    fontmult: u8,
    /// Pixel shift FBInk silently adds to every print at the current fontmult
    /// (dead-zone centering + viewport origins). Subtracted from
    /// hoffset/voffset so glyphs land exactly at the requested coordinates.
    pen_shift: (i32, i32),
}

fn check(ret: i32, what: &str) -> io::Result<()> {
    if ret < 0 {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("FBInk {what} failed ({ret})"),
        ))
    } else {
        Ok(())
    }
}

fn base_config() -> sys::FBInkConfig {
    let mut cfg = sys::FBInkConfig::default();
    cfg.is_quiet = true;
    // We batch draws and refresh once per frame ourselves.
    cfg.no_refresh = true;
    // Don't let FBInk shift rows down to vertically balance the cell grid;
    // we position by pixel, not by row/col. (Honored at init time.)
    cfg.no_viewport = true;
    cfg
}

/// The horizontal/vertical pixel shift FBInk will apply to prints under the
/// given (already-initialized) config. Horizontal: the dead-zone centering
/// `(viewWidth - MAXCOLS*FONTW) / 2` when columns don't fit perfectly, plus
/// the viewport origin. Vertical: the viewport origin (which folds in the
/// row-balancing offset when `no_viewport` is off).
fn pen_shift(cfg: &sys::FBInkConfig) -> (i32, i32) {
    let mut state = sys::FBInkState::default();
    unsafe {
        sys::fbink_get_state(cfg, &mut state);
    }
    let deadzone = if state.is_perfect_fit {
        0
    } else {
        (state.view_width as i32 - state.max_cols as i32 * state.font_w as i32) / 2
    };
    (
        deadzone + state.view_hori_origin as i32,
        state.view_vert_origin as i32,
    )
}

impl FbinkRenderer {
    /// Opens the framebuffer, initializes FBInk at fontmult=1 (matching
    /// [`crate::widget::font`] cell metrics), and reads the panel geometry.
    pub fn open() -> io::Result<Self> {
        unsafe {
            let fbfd = sys::fbink_open();
            if fbfd < 0 {
                return Err(io::Error::new(io::ErrorKind::Other, "fbink_open failed"));
            }
            // Explicit 1x — fontmult=0 means "auto" and picks a DPI-scaled size
            // that would disagree with our layout measurements.
            let mut init_cfg = base_config();
            init_cfg.fontmult = 1;
            check(sys::fbink_init(fbfd, &init_cfg), "init")?;

            let mut state = sys::FBInkState::default();
            sys::fbink_get_state(&init_cfg, &mut state);
            let screen = Size::new(state.view_width as i32, state.view_height as i32);

            Ok(FbinkRenderer {
                fbfd,
                screen,
                fontmult: 1,
                pen_shift: pen_shift(&init_cfg),
            })
        }
    }

    /// Re-runs `fbink_init` when `size` differs from the live multiplier.
    /// Required because FBInk ignores `fontmult` on print/cls/refresh.
    fn ensure_fontmult(&mut self, size: u8) -> io::Result<()> {
        let size = size.max(1);
        if self.fontmult == size {
            return Ok(());
        }
        let mut cfg = base_config();
        cfg.fontmult = size;
        unsafe {
            check(sys::fbink_init(self.fbfd, &cfg), "init(fontmult)")?;
        }
        self.fontmult = size;
        // The dead-zone shift depends on the cell width, i.e. on fontmult.
        self.pen_shift = pen_shift(&cfg);
        Ok(())
    }

    fn set_pens(&self, fg: Color, bg: Color) -> io::Result<()> {
        unsafe {
            // (gray, quantize=false, update=true): apply to the live pen now.
            check(
                sys::fbink_set_fg_pen_gray(fg.to_gray8(), false, true),
                "set_fg_pen",
            )?;
            check(
                sys::fbink_set_bg_pen_gray(bg.to_gray8(), false, true),
                "set_bg_pen",
            )?;
        }
        Ok(())
    }

    fn draw_text(
        &mut self,
        x: i32,
        y: i32,
        text: &str,
        size: u8,
        fg: Color,
        bg: Color,
        inverse: bool,
    ) -> io::Result<()> {
        self.ensure_fontmult(size)?;
        self.set_pens(fg, bg)?;
        let mut cfg = base_config();
        cfg.fontmult = self.fontmult;
        // Counteract FBInk's implicit pen shift so the glyphs land exactly at
        // (x, y) — the same raw coordinates cls fills and refresh flushes.
        cfg.hoffset = (x - self.pen_shift.0) as i16;
        cfg.voffset = (y - self.pen_shift.1) as i16;
        cfg.is_inverted = inverse;
        let c = CString::new(text).unwrap_or_default();
        unsafe { check(sys::fbink_print(self.fbfd, c.as_ptr(), &cfg), "print") }
    }

    fn fill(&self, rect: Rect, color: Color) -> io::Result<()> {
        // fbink_cls clears the rectangle to the background pen.
        self.set_pens(color, color)?;
        let cfg = base_config();
        let r = sys::FBInkRect {
            left: rect.x.max(0) as u16,
            top: rect.y.max(0) as u16,
            width: rect.w.max(0) as u16,
            height: rect.h.max(0) as u16,
        };
        unsafe { check(sys::fbink_cls(self.fbfd, &cfg, &r, false), "cls") }
    }
}

impl Drop for FbinkRenderer {
    fn drop(&mut self) {
        unsafe {
            sys::fbink_close(self.fbfd);
        }
    }
}

impl Renderer for FbinkRenderer {
    fn screen_size(&mut self) -> Size {
        self.screen
    }

    fn submit(&mut self, cmds: &[DrawCmd]) -> io::Result<()> {
        for cmd in cmds {
            match cmd {
                DrawCmd::FillRect { rect, color } => self.fill(*rect, *color)?,
                DrawCmd::Text {
                    x,
                    y,
                    text,
                    size,
                    fg,
                    bg,
                    inverse,
                } => self.draw_text(*x, *y, text, *size, *fg, *bg, *inverse)?,
            }
        }
        Ok(())
    }

    fn refresh(&mut self, region: Rect, mode: RefreshMode) -> io::Result<()> {
        let mut cfg = base_config();
        // A flashing update is FBInk's full (UPDATE_MODE_FULL) refresh, which
        // clears ghosting; a non-flashing update is the fast partial path.
        cfg.is_flashing = matches!(mode, RefreshMode::Full);
        unsafe {
            check(
                sys::fbink_refresh(
                    self.fbfd,
                    region.y.max(0) as u32,
                    region.x.max(0) as u32,
                    region.w.max(0) as u32,
                    region.h.max(0) as u32,
                    &cfg,
                ),
                "refresh",
            )
        }
    }
}
