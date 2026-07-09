# Usage

How to build screens with the Rust API. For the reactive model, see
[concepts.md](concepts.md). For FBInk / touch / `App::run`, see
[device.md](device.md).

## Dependencies

```toml
[dependencies]
signalkit = { path = "../signalkit" }          # host / tests
# signalkit = { path = "../signalkit", features = ["fbink"] }  # on device
```

Import the common surface:

```rust
use signalkit::{
    for_each, hstack, slot, vstack, Align, Axis, BuildCtx, Button, Color,
    Component, ExitHandle, IntoNode, Label, Node, Signal, Spacer,
};
```

## Signals

```rust
let count = Signal::new(0);
assert_eq!(count.get(), 0);

count.set(1);
count.set_if_changed(1); // no-op, observers not notified
count.update(|v| v + 1);

count.with(|v| println!("{v}"));

let _sub = count.observe(|v| println!("now {v}"));
// Drop or dispose to unsubscribe. Prefer BuildCtx::bind inside components.
```

`T` must be `'static`. `get` / write coalescing need `Clone`.

## A component

```rust
struct Screen {
    title: Signal<String>,
}

impl Component for Screen {
    fn build(&mut self, ctx: &mut BuildCtx) -> Node {
        let label = Label::new("");
        {
            let label = label.clone();
            ctx.bind(&self.title, move |t| label.set_text(t.clone()));
        }
        label.into_node()
    }

    fn did_mount(&mut self) {
        // Optional: start timers, open files, etc.
    }

    fn will_unmount(&mut self) {
        // Optional: explicit cleanup beyond tracked disposables.
    }
}
```

`ctx.bind` runs the closure immediately with the current value, then on every
change. Capture **cloned** widget handles (and cloned signals) inside the
closure — the widget types are `Rc`-backed and cheap to clone.

## Widgets

### Label

Single-line text. Size is an FBInk font multiplier (`1` = base cell, typically
8×16).

```rust
let label = Label::new("Hello")
    .size(2)
    .colors(Color::BLACK, Color::WHITE);

label.set_text("Updated");
```

### Button

Filled chip with inverted title text. Tap handler is a `Fn()` closure.

```rust
let count = count.clone();
Button::new(" + ")
    .size(3)
    .on_tap(move || count.update(|v| v + 1))
```

### Spacer

Flexible empty space along the stack’s main axis. Absorbs leftover room after
siblings take their natural size.

```rust
vstack(8, Align::Fill, vec![
    Label::new("Top").into_node(),
    Spacer::new().into_node(),
    Label::new("Bottom").into_node(),
])
```

### Stack

Usually created via `vstack` / `hstack`. You can also build a `Node::Stack`
when you need padding or a background:

```rust
Node::Stack {
    axis: Axis::Vertical,
    spacing: 20,
    padding: 24,
    align: Align::Center,
    bg: Some(Color::WHITE),
    children: vec![/* ... */],
}
```

### Alignment

Cross-axis placement of children inside a stack:

| `Align` | Behavior |
|---------|----------|
| `Fill` | Child stretched to cross-axis size |
| `Leading` | Top (horizontal stack) / left (vertical) |
| `Center` | Centered on the cross axis |
| `Trailing` | Bottom / right |

## Layout helpers

```rust
vstack(spacing, align, children)  // vertical
hstack(spacing, align, children)  // horizontal
group(children)                   // flatten into parent (no extra stack)
```

Children are `Node`s. Widgets and `Box<dyn Component>` convert via `IntoNode`:

```rust
vstack(12, Align::Leading, vec![
    Label::new("A").into_node(),
    Button::new("B").into_node(),
    Box::new(ChildComponent { /* ... */ }).into_node(),
])
```

Layout is a single-pass flexbox-lite: measure natural sizes, place along the
main axis with spacing, give leftover main-axis space to flexible spacers,
then place on the cross axis per `Align`. Frames are absolute screen pixels.

Font metrics used for measure must match what FBInk draws — see
[device.md](device.md#fonts).

## Binding patterns

**Label text from a signal:**

```rust
let label = Label::new("");
{
    let label = label.clone();
    ctx.bind(&self.count, move |v| label.set_text(format!("Count: {v}")));
}
```

**Button that writes a signal:**

```rust
let count = self.count.clone();
Button::new("Reset").on_tap(move || count.set(0))
```

**Observe without applying immediately:**

```rust
ctx.observe(&self.flag, false, move |v| {
    // only on subsequent changes
    let _ = v;
});
```

**Child component:**

```rust
let child_root = ctx.host(Box::new(OtherScreen { /* ... */ }));
// child_root is an AnyWidget; usually you return it via Node::Widget
// or embed OtherScreen as Node::Component in a stack instead.
```

Embedding as a node is usually clearer:

```rust
vstack(8, Align::Fill, vec![
    Label::new("Parent").into_node(),
    Box::new(OtherScreen { /* ... */ }).into_node(),
])
```

## `slot` — swap one subtree

When a signal’s value should replace the entire child UI:

```rust
use signalkit::slot;

let body = slot(&self.mode, |mode| match mode {
    Mode::A => Label::new("Mode A").into_node(),
    Mode::B => Button::new("Mode B").into_node(),
});
```

The previous child is unmounted (observers disposed) before the new one mounts.
For per-value reactive content inside the slot, return a `Node::Component`.

## `for_each` — keyed lists

```rust
use signalkit::for_each;

#[derive(Clone)]
struct Row { id: u32, text: String }

let list = for_each(
    &self.rows,           // Signal<Vec<Row>>
    |r| r.id,             // stable key
    |r| Label::new(r.text.clone()).into_node(),
);
```

Rules:

- Keys must be unique; duplicates **panic**.
- Missing keys are unmounted; new keys are mounted; order follows the `Vec`.
- The item builder runs for new keys; existing keys keep their mounted
  instance (they do not rebuild when only other fields of the row change —
  put row-local signals/bindings inside a child component if you need that).

Mutating the collection:

```rust
rows.update(|current| {
    let mut v = current.clone();
    v.push(Row { id: next, text: format!("Row {next}") });
    v
});
```

## Exit handle

Share an `ExitHandle` with any button (or other code) that should stop the
event loop:

```rust
let exit = ExitHandle::new();
let btn = {
    let exit = exit.clone();
    Button::new("Exit").on_tap(move || exit.request_exit())
};
// pass the same exit into App::new(..., exit)
```

## Driving the UI without `run`

Useful in tests and host stubs:

```rust
use signalkit::geometry::{Point, Size};
use signalkit::render::mock::MockRenderer;
use signalkit::App;

let exit = ExitHandle::new();
let mut app = App::new(
    Box::new(Screen { /* ... */ }),
    MockRenderer::new(Size::new(600, 800)),
    exit,
);

app.render_frame()?;
app.tap_at(Point::new(100, 200));
app.render_frame()?;
```

`MockRenderer` records draw commands and refreshes for assertions.

## Full example

See [`signalkit-demo`](../../signalkit-demo/src/main.rs): counter, `for_each`
list, add-row button, and exit — the same structure as a real Kindle screen.
