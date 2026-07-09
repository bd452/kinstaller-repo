//! The application: mounts a root component, renders frames, and (on device)
//! runs the touch event loop.
//!
//! This is the piece SignalKit does not have — UIKit owns the run loop and
//! render server there. Here the loop is explicit: a touch wakes us, we
//! hit-test it, dispatch a tap, let the resulting signal writes mutate widgets,
//! then render exactly the damaged region with an appropriate e-ink refresh.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::component::{mount_component, Component, Mounted};
use crate::geometry::{Point, Rect, Size};
use crate::layout::layout;
use crate::render::{Color, DrawCmd, RefreshMode, Renderer};
use crate::widget::{hit_test, walk, WidgetId};

/// Force a full refresh when the damaged area exceeds this percentage of the
/// screen (a large change is cheaper and cleaner as one full flash).
const FULL_REFRESH_AREA_PCT: i64 = 40;

/// Force a full refresh when enough distinct screen tiles have been touched by
/// partial refreshes since the last full flash.
const FULL_REFRESH_DIRTY_TILE_PCT: i64 = 30;

/// Safety ceiling for tiny partial updates whose debt never quite reaches the
/// threshold on a particular screen.
const MAX_PARTIAL_REFRESHES: u32 = 32;

const DAMAGE_TILE_COLS: usize = 8;
const DAMAGE_TILE_ROWS: usize = 8;
const DAMAGE_TILE_COUNT: usize = DAMAGE_TILE_COLS * DAMAGE_TILE_ROWS;

/// A shared exit flag. Give a clone to any component (e.g. an "Exit" button's
/// handler) so it can ask the event loop to stop.
#[derive(Clone, Default)]
pub struct ExitHandle(Rc<Cell<bool>>);

impl ExitHandle {
    pub fn new() -> Self {
        ExitHandle(Rc::new(Cell::new(false)))
    }

    /// Requests the event loop stop after the current frame.
    pub fn request_exit(&self) {
        self.0.set(true);
    }

    pub fn is_set(&self) -> bool {
        self.0.get()
    }
}

/// Owns the mounted UI, the renderer, and the frame/refresh bookkeeping.
///
/// Generic over the [`Renderer`] so tests can drive it with
/// [`MockRenderer`](crate::render::mock::MockRenderer) and inspect the output.
pub struct App<R: Renderer> {
    root: Mounted,
    pub(crate) renderer: R,
    screen: Size,
    // Read by the on-device event loop (`run`, behind the `fbink` feature).
    #[cfg_attr(not(feature = "fbink"), allow(dead_code))]
    exit: ExitHandle,
    /// Frame each widget occupied at the last paint (for damage of moves/removals).
    last_frames: HashMap<WidgetId, Rect>,
    partial_since_full: u32,
    dirty_tiles: [bool; DAMAGE_TILE_COUNT],
    dirty_tile_count: usize,
    first_frame: bool,
}

impl<R: Renderer> App<R> {
    /// Mounts `root` and prepares to render onto `renderer`. Share `exit` with
    /// components that need to stop the loop.
    pub fn new(root: Box<dyn Component>, mut renderer: R, exit: ExitHandle) -> Self {
        let screen = renderer.screen_size();
        App {
            root: mount_component(root),
            renderer,
            screen,
            exit,
            last_frames: HashMap::new(),
            partial_since_full: 0,
            dirty_tiles: [false; DAMAGE_TILE_COUNT],
            dirty_tile_count: 0,
            first_frame: true,
        }
    }

    fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.screen.w, self.screen.h)
    }

    /// Hit-tests `point` and, on a touch release, fires the tap handler of the
    /// widget under it. Returns true if a handler ran. Signal writes made by the
    /// handler mark widgets dirty; call [`render_frame`](Self::render_frame)
    /// afterwards to flush them.
    pub fn tap_at(&mut self, point: Point) -> bool {
        let root = self.root.root();
        if let Some(target) = hit_test(&root, point) {
            return target.dispatch_tap();
        }
        false
    }

    fn tiles_for_damage(&self, damage: Rect, bounds: Rect) -> Vec<usize> {
        let damage = damage.intersection(bounds);
        if damage.is_empty() || bounds.is_empty() {
            return Vec::new();
        }

        let cols = DAMAGE_TILE_COLS as i64;
        let rows = DAMAGE_TILE_ROWS as i64;
        let width = bounds.w.max(1) as i64;
        let height = bounds.h.max(1) as i64;

        let left = ((damage.x - bounds.x).max(0) as i64 * cols / width).clamp(0, cols - 1) as usize;
        let right = (((damage.right() - 1 - bounds.x).max(0) as i64) * cols / width)
            .clamp(0, cols - 1) as usize;
        let top = ((damage.y - bounds.y).max(0) as i64 * rows / height).clamp(0, rows - 1) as usize;
        let bottom = (((damage.bottom() - 1 - bounds.y).max(0) as i64) * rows / height)
            .clamp(0, rows - 1) as usize;

        let mut tiles = Vec::new();
        for row in top..=bottom {
            for col in left..=right {
                tiles.push(row * DAMAGE_TILE_COLS + col);
            }
        }
        tiles
    }

    fn dirty_tile_count_after_partial(&self, damage: Rect, bounds: Rect) -> usize {
        let mut count = self.dirty_tile_count;
        for tile in self.tiles_for_damage(damage, bounds) {
            if !self.dirty_tiles[tile] {
                count += 1;
            }
        }
        count
    }

    fn record_partial_damage(&mut self, damage: Rect, bounds: Rect) {
        for tile in self.tiles_for_damage(damage, bounds) {
            if !self.dirty_tiles[tile] {
                self.dirty_tiles[tile] = true;
                self.dirty_tile_count += 1;
            }
        }
    }

    /// Re-lays-out the tree, computes the damaged region, redraws it, and issues
    /// one e-ink refresh (partial, or full to clear ghosting per the policy).
    pub fn render_frame(&mut self) -> std::io::Result<()> {
        let root = self.root.root();
        let bounds = self.bounds();

        // Damage from widgets currently dirty, at their pre-layout frames.
        let mut damage = Rect::ZERO;
        walk(&root, &mut |w| {
            if w.is_dirty() {
                if let Some(old) = self.last_frames.get(&w.id()) {
                    damage = damage.union(*old);
                }
            }
        });

        layout(&root, bounds);

        // Damage from dirty / moved / newly-added widgets, and snapshot frames.
        let mut new_frames = HashMap::new();
        let mut current_ids = HashSet::new();
        walk(&root, &mut |w| {
            let id = w.id();
            let f = w.frame();
            new_frames.insert(id, f);
            current_ids.insert(id);
            let old = self.last_frames.get(&id).copied();
            let moved = old != Some(f);
            if w.is_dirty() || moved || old.is_none() {
                damage = damage.union(f);
                if let Some(old) = old {
                    damage = damage.union(old);
                }
            }
        });
        // Damage from widgets that were removed since the last frame.
        for (id, old) in &self.last_frames {
            if !current_ids.contains(id) {
                damage = damage.union(*old);
            }
        }

        let area_forces_full = damage.area() * 100 >= bounds.area() * FULL_REFRESH_AREA_PCT;
        let counter_forces_full = self.partial_since_full + 1 >= MAX_PARTIAL_REFRESHES;
        let dirty_tiles_after_partial = self.dirty_tile_count_after_partial(damage, bounds);
        let tile_coverage_forces_full = dirty_tiles_after_partial as i64 * 100
            >= DAMAGE_TILE_COUNT as i64 * FULL_REFRESH_DIRTY_TILE_PCT;

        let mut cmds: Vec<DrawCmd> = Vec::new();
        if !self.first_frame
            && !area_forces_full
            && !counter_forces_full
            && !tile_coverage_forces_full
            && !damage.is_empty()
        {
            // Erase the damaged region, then repaint every widget touching it.
            cmds.push(DrawCmd::FillRect {
                rect: damage,
                color: Color::WHITE,
            });
            walk(&root, &mut |w| {
                if w.frame().intersects(damage) {
                    w.paint_self(&mut cmds);
                }
            });
            // Stack backgrounds can extend far beyond the damage (the root's
            // covers the whole screen). Clip fills to the damage so we never
            // blank framebuffer pixels that won't be repainted and refreshed
            // this frame.
            for cmd in &mut cmds {
                if let DrawCmd::FillRect { rect, .. } = cmd {
                    *rect = rect.intersection(damage);
                }
            }
            cmds.retain(|c| !matches!(c, DrawCmd::FillRect { rect, .. } if rect.is_empty()));
        }

        let full = self.first_frame
            || area_forces_full
            || counter_forces_full
            || tile_coverage_forces_full;

        if full {
            cmds.clear();
            cmds.push(DrawCmd::FillRect {
                rect: bounds,
                color: Color::WHITE,
            });
            walk(&root, &mut |w| w.paint_self(&mut cmds));
            self.renderer.submit(&cmds)?;
            self.renderer.refresh(bounds, RefreshMode::Full)?;
            self.partial_since_full = 0;
            self.dirty_tiles = [false; DAMAGE_TILE_COUNT];
            self.dirty_tile_count = 0;
            self.first_frame = false;
        } else if !damage.is_empty() {
            self.renderer.submit(&cmds)?;
            self.renderer.refresh(damage, RefreshMode::Partial)?;
            self.partial_since_full += 1;
            self.record_partial_damage(damage, bounds);
        }

        walk(&root, &mut |w| w.clear_dirty());
        self.last_frames = new_frames;
        Ok(())
    }
}

/// On-device touch event loop.
#[cfg(feature = "fbink")]
impl<R: Renderer> App<R> {
    /// Renders the first frame, then blocks reading touch events until an
    /// [`ExitHandle`] fires. Each release dispatches a tap and renders a frame.
    ///
    /// The touchscreen is opened under an exclusive `EVIOCGRAB` so Kindle's
    /// framework/home screen does not also receive the taps (without the grab,
    /// events fall through to whatever is underneath our framebuffer drawing).
    ///
    /// Sleep/lock ownership is left to the framework once this runs as a
    /// booklet; this loop does not try to cooperate with the screensaver.
    pub fn run(&mut self) -> std::io::Result<()> {
        use crate::input::{find_touch_device, Calibration, GrabbedTouch, TouchDecoder, TouchKind};

        self.render_frame()?;

        let dev = find_touch_device().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no touchscreen found (set SIGNALKIT_TOUCH_DEV)",
            )
        })?;
        // Grab for the lifetime of this scope; Drop releases it on exit/error.
        let mut touch = GrabbedTouch::open(&dev)?;
        // Scale raw digitizer coords → screen pixels via EVIOCGABS maxima.
        let calib = Calibration::from_device(self.screen, touch.axis_max());
        let mut decoder = TouchDecoder::new();
        let mut record = [0u8; crate::input::evdev::EVENT_SIZE];

        while !self.exit.is_set() {
            touch.read_record(&mut record)?;
            if let Some(ev) = decoder.feed(&record) {
                if ev.kind == TouchKind::Up {
                    let p = calib.map(ev.x, ev.y);
                    if self.tap_at(p) {
                        self.render_frame()?;
                    }
                }
            }
        }
        // Drop the grab before restoring the framework UI so it can take input.
        drop(touch);
        // Don't white-out the framebuffer here: the Kindle framework still owns
        // its scene and won't repaint until we ask it to. Hand control back.
        restore_kindle_ui();
        Ok(())
    }
}

/// Asks the Kindle framework to show the home booklet again and re-enable the
/// status-bar overlay (pillow). Best-effort: failures are ignored so a missing
/// `lipc-set-prop` on a non-Kindle host never aborts exit.
#[cfg(feature = "fbink")]
fn restore_kindle_ui() {
    use std::process::Command;
    let _ = Command::new("lipc-set-prop")
        .args(["com.lab126.pillow", "disableEnablePillow", "enable"])
        .status();
    let _ = Command::new("lipc-set-prop")
        .args([
            "com.lab126.appmgrd",
            "start",
            "app://com.lab126.booklet.home",
        ])
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::BuildCtx;
    use crate::node::{vstack, IntoNode};
    use crate::render::mock::MockRenderer;
    use crate::signal::Signal;
    use crate::widget::{Align, AnyWidget, Button, Label};

    struct Screen {
        count: Signal<i32>,
    }
    impl Component for Screen {
        fn build(&mut self, ctx: &mut BuildCtx) -> Node {
            let label = Label::new("");
            let l = label.clone();
            ctx.bind(&self.count, move |v| l.set_text(format!("{v}")));
            let inc = self.count.clone();
            let button = Button::new("+").on_tap(move || inc.update(|v| v + 1));
            vstack(4, Align::Fill, vec![label.into_node(), button.into_node()])
        }
    }
    use crate::node::Node;

    fn app_with(count: Signal<i32>) -> App<MockRenderer> {
        App::new(
            Box::new(Screen { count }),
            MockRenderer::new(Size::new(600, 800)),
            ExitHandle::new(),
        )
    }

    #[test]
    fn first_frame_is_a_full_refresh() {
        let mut app = app_with(Signal::new(0));
        app.render_frame().unwrap();
        assert_eq!(app.renderer.refreshes.len(), 1);
        assert_eq!(app.renderer.refreshes[0].1, RefreshMode::Full);
        assert_eq!(app.renderer.refreshes[0].0, app.bounds());
    }

    #[test]
    fn small_change_does_a_partial_refresh_of_the_damage() {
        let count = Signal::new(0);
        let mut app = app_with(count.clone());
        app.render_frame().unwrap(); // full
        app.renderer.clear_log();

        count.set(1);
        app.render_frame().unwrap();
        assert_eq!(app.renderer.refreshes.len(), 1);
        assert_eq!(app.renderer.refreshes[0].1, RefreshMode::Partial);
        // Damage is confined to the label, not the whole screen.
        assert!(app.renderer.refreshes[0].0.area() < app.bounds().area());
    }

    #[test]
    fn repeated_partial_damage_in_same_area_stays_partial_until_ceiling() {
        let count = Signal::new(0);
        let mut app = app_with(count.clone());
        app.render_frame().unwrap(); // full, resets counter

        for i in 1..MAX_PARTIAL_REFRESHES {
            app.renderer.clear_log();
            count.set(i as i32);
            app.render_frame().unwrap();
            assert_eq!(app.renderer.refreshes[0].1, RefreshMode::Partial);
        }
    }

    #[test]
    fn distributed_partial_damage_forces_full_refresh() {
        let count = Signal::new(0);
        let mut app = app_with(count.clone());
        app.render_frame().unwrap(); // full, resets coverage

        // Pretend previous partial refreshes have already touched many
        // different tiles. The next small update should cross the coverage
        // threshold and promote to a full flash.
        let needed = (DAMAGE_TILE_COUNT as i64 * FULL_REFRESH_DIRTY_TILE_PCT + 99) / 100;
        for tile in 1..needed as usize {
            app.dirty_tiles[tile] = true;
            app.dirty_tile_count += 1;
        }

        assert!(!app.dirty_tiles[0]);
        app.renderer.clear_log();
        count.set(1);
        app.render_frame().unwrap();
        assert_eq!(app.renderer.refreshes[0].1, RefreshMode::Full);
        assert_eq!(app.dirty_tile_count, 0);
    }

    /// Like the demo's root: a full-screen stack with a solid background.
    struct BgScreen {
        count: Signal<i32>,
    }
    impl Component for BgScreen {
        fn build(&mut self, ctx: &mut BuildCtx) -> Node {
            let label = Label::new("");
            let l = label.clone();
            ctx.bind(&self.count, move |v| l.set_text(format!("{v}")));
            Node::Stack {
                axis: crate::widget::Axis::Vertical,
                spacing: 4,
                padding: 0,
                align: Align::Fill,
                bg: Some(Color::WHITE),
                children: vec![label.into_node()],
            }
        }
    }

    #[test]
    fn partial_frame_fills_are_clipped_to_the_damage() {
        let count = Signal::new(0);
        let mut app = App::new(
            Box::new(BgScreen {
                count: count.clone(),
            }),
            MockRenderer::new(Size::new(600, 800)),
            ExitHandle::new(),
        );
        app.render_frame().unwrap(); // full
        app.renderer.clear_log();

        count.set(1);
        app.render_frame().unwrap();
        let (damage, mode) = app.renderer.refreshes[0];
        assert_eq!(mode, RefreshMode::Partial);
        // The root stack's full-screen background intersects the damage, but
        // its fill must not blank framebuffer pixels outside the refresh.
        for cmd in &app.renderer.commands {
            if let DrawCmd::FillRect { rect, .. } = cmd {
                assert!(
                    damage.contains_rect(*rect),
                    "fill {rect:?} escapes damage {damage:?}"
                );
            }
        }
    }

    #[test]
    fn tap_dispatch_updates_state() {
        let count = Signal::new(0);
        let mut app = app_with(count.clone());
        app.render_frame().unwrap();
        // Button sits below the label; tap within its frame.
        let root = app.root.root();
        let button_frame = root.children()[1].frame();
        let hit = Point::new(
            button_frame.x + button_frame.w / 2,
            button_frame.y + button_frame.h / 2,
        );
        assert!(app.tap_at(hit));
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn tap_on_empty_space_is_ignored() {
        let mut app = app_with(Signal::new(0));
        app.render_frame().unwrap();
        assert!(!app.tap_at(Point::new(599, 799)));
    }

    // Silence unused import warning for AnyWidget in some cfgs.
    #[allow(dead_code)]
    fn _use(_: AnyWidget) {}
}
