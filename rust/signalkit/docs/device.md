# On-device: rendering, touch, and the event loop

Enable the `fbink` feature to link FBInk and unlock the Kindle backends.

```toml
signalkit = { path = "../signalkit", features = ["fbink"] }
```

## Opening the app

```rust
use signalkit::render::fbink::FbinkRenderer;
use signalkit::{App, ExitHandle};

let exit = ExitHandle::new();
let renderer = FbinkRenderer::open()?;
let mut app = App::new(root(exit.clone()), renderer, exit);
app.run()?;
```

`App::run`:

1. Renders the first frame (full refresh).
2. Opens the touchscreen under an exclusive `EVIOCGRAB`.
3. Blocks reading evdev records; on touch-up, hit-tests and dispatches taps.
4. After each handled tap, calls `render_frame`.
5. When `ExitHandle` is set, releases the grab and asks the framework to show
   home again (`lipc-set-prop` → pillow enable + `appmgrd` home booklet).

## Renderer contract

```rust
trait Renderer {
    fn screen_size(&mut self) -> Size;
    fn submit(&mut self, cmds: &[DrawCmd]) -> io::Result<()>;
    fn refresh(&mut self, region: Rect, mode: RefreshMode) -> io::Result<()>;
}
```

Per frame the app:

1. Unions dirty / moved / removed widget frames into a damage rect.
2. Runs layout.
3. Either full-paints the screen or clears + paints widgets intersecting damage.
4. Calls `submit`, then a single `refresh` for that region.

### Draw commands

| `DrawCmd` | Meaning |
|-----------|---------|
| `FillRect { rect, color }` | Solid 4-bit gray fill (`Color` 0–15; black=0, white=15) |
| `Text { x, y, text, size, fg, bg, inverse }` | Bitmap text at top-left; `size` is font multiplier |

### Refresh policy

| Mode | When |
|------|------|
| `Full` | First frame; damage ≥ 40% of screen area; accumulated ghost debt over threshold; or a high safety ceiling of partial refreshes |
| `Partial` | Small updates (faster, may ghost slightly) |

Ghost debt is a heuristic based on damaged area, repeated updates to the same
region, text redraws, inverse text, and non-white fills. A full refresh resets
the debt. Constants live in `app.rs` (`FULL_REFRESH_AREA_PCT`,
`GHOST_DEBT_LIMIT`, `MAX_PARTIAL_REFRESHES`).

## Fonts

Layout measures text with an 8×16 cell at multiplier 1 (IBM VGA-style metrics
shared with FBInk). **FBInk only honors `fontmult` at `fbink_init` time**, not
on each print. The FBInk backend tracks the active multiplier and re-inits when
a draw needs a different size so measured frames match painted glyphs.

If text overflows buttons or leaves ghost edges after updates, the usual cause
is a metrics/renderer mismatch — not layout math alone.

## Touch input

### Device discovery

Looks for an evdev node with multitouch absolute axes. Override with:

```sh
export SIGNALKIT_TOUCH_DEV=/dev/input/event2
```

### Grab

`EVIOCGRAB` prevents taps from falling through to the home screen underneath
the framebuffer. The grab is held for the life of `App::run` and released on
drop / exit.

### Calibration

Raw digitizer ranges often differ from panel pixels. At startup the loop
queries `EVIOCGABS` for `ABS_MT_POSITION_X/Y` and builds
`Calibration::from_device`. Optional env overrides:

| Variable | Effect |
|----------|--------|
| `SIGNALKIT_TOUCH_SWAP=1` | Swap X/Y |
| `SIGNALKIT_TOUCH_INVERT_X=1` | Invert X after swap |
| `SIGNALKIT_TOUCH_INVERT_Y=1` | Invert Y after swap |
| `SIGNALKIT_TOUCH_RAW=WxH` | Force raw axis maxima |

### Decoding

Hand-rolled Linux `input_event` parsing (protocol B with A fallback):
`ABS_MT_*`, `BTN_TOUCH`, syn reports. Only **touch-up** dispatches a tap
(press-and-hold does not fire until release).

## Exit and restoring the Kindle UI

Clearing the framebuffer is not enough — the framework will not repaint home
on its own after a raw process exits. On exit, SignalKit (and the demo’s
`launch.sh`) call:

```sh
lipc-set-prop com.lab126.pillow disableEnablePillow enable
lipc-set-prop com.lab126.appmgrd start app://com.lab126.booklet.home
```

The demo `launch.sh` also disables pillow while the app runs so the status bar
does not draw over the UI.

## Sleep / lock screen

A process that grabs touch and paints `/dev/fb0` is **not** a Kindle booklet.
The stock screensaver and “swipe to unlock” UI expect framework ownership of
input and the panel. Manual launches will fight that stack.

**Intended direction:** register and launch as an `appmgrd` booklet so the
system manages foregrounding around sleep. SignalKit does not currently
implement booklet lifecycle hooks; do not expect lock-screen cooperation from
`App::run` alone.

## Host stub without FBInk

```rust
#[cfg(not(feature = "fbink"))]
fn main() -> std::io::Result<()> {
    use signalkit::geometry::Size;
    use signalkit::render::mock::MockRenderer;
    use signalkit::App;

    let exit = ExitHandle::new();
    let mut app = App::new(root(exit.clone()), MockRenderer::new(Size::new(600, 800)), exit);
    app.render_frame()?;
    Ok(())
}
```

This is what `signalkit-demo` does when built without `fbink`.
