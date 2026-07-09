# Concepts

SignalKit is a **build-once, mutate-in-place** UI framework. There is no virtual
DOM and no re-render of the component tree when state changes. That design comes
from the original Swift SignalKit (which sits on UIKit); the Kindle port keeps
the same model and adds the pieces UIKit would have provided.

## Mental model

1. A **component** implements `build`, which runs **once** at mount and returns
   a `Node` tree (widgets, stacks, child components, `slot` / `for_each`).
2. During `build`, you register **bindings** and **observers** on signals via
   `BuildCtx`. Those closures typically capture widget handles and call setters
   like `Label::set_text`.
3. Later, a signal write runs those closures **synchronously**. Setters mark
   widgets dirty. The app lays out if needed, paints the damaged region, and
   refreshes the e-ink panel.

```
Signal::set ──► observer closures ──► widget setters (dirty)
                                          │
App::render_frame ◄── damage union ◄──────┘
       │
       ├── layout (frames)
       ├── submit DrawCmds
       └── refresh (partial or full)
```

## Signals

`Signal<T>` is a cloneable handle (`Rc<RefCell<…>>`) to a shared value plus a
set of observers. It is single-threaded (`!Send` / `!Sync`), matching the
original `@MainActor` isolation. The on-device event loop is the moral
equivalent of the main actor — do not touch signals or widgets from other
threads.

Writes notify observers immediately. If an observer writes the same signal
again (re-entrancy), delivery does **not** nest: the write is coalesced and a
single follow-up pass runs with the final value. That matches Swift SignalKit.

Useful methods:

| Method | Role |
|--------|------|
| `get` / `with` | Read (clone vs borrow) |
| `set` / `set_if_changed` / `update` | Write |
| `observe` | Subscribe; returns a `Disposable` |

Prefer `BuildCtx::bind` / `observe` inside components so subscriptions are
tied to the component lifetime and torn down on unmount.

## Components and `BuildCtx`

```rust
pub trait Component {
    fn build(&mut self, ctx: &mut BuildCtx) -> Node;
    fn did_mount(&mut self) {}
    fn will_unmount(&mut self) {}
}
```

`BuildCtx` replaces the services the Swift base class offered:

| Method | Role |
|--------|------|
| `bind(signal, apply)` | Fire `apply` now and on every change (typical widget binding) |
| `observe(signal, fire_immediately, handler)` | Lower-level subscription |
| `track(disposable)` | Keep a disposable alive until unmount |
| `on_cleanup(f)` | Run `f` at unmount |
| `host(child)` | Mount a child component; lifetime tied to this scope |

Teardown order is owned by an internal `LifecycleScope`: cleanup handlers,
then observers, then child mounts, then widgets. You normally do not touch
this directly.

## Nodes vs widgets

- A **`Node`** is a *description* returned from `build`. It is consumed at
  mount and turned into widgets.
- A **widget** (`Label`, `Button`, `Spacer`, `Stack`, wrapped as `AnyWidget`)
  is a retained, cloneable handle. After mount, you mutate it through setters;
  you do not rebuild the `Node` tree (except via `slot` / `for_each`).

`vstack` / `hstack` / `group` and `IntoNode` exist so composition reads like
declarative UI without a result builder.

## Structural updates

Almost everything is static after mount. Two helpers change the tree later:

- **`slot`** — one child, remounted whenever a signal changes.
- **`for_each`** — keyed list over `Signal<Vec<T>>`; mounts/unmounts/reorders
  children by key. Duplicate keys panic.

See [usage.md](usage.md) for examples.

## Rendering and input (Kindle additions)

The original framework never drew pixels. This port adds:

- **`Renderer`** — `submit` draw commands, then `refresh` a region
  (`Partial` or `Full`).
- **Damage tracking** — dirty widgets and moved/removed frames are unioned;
  only that region is cleared and redrawn.
- **Touch** — Linux evdev multitouch, optional exclusive grab, calibration
  into screen pixels, hit-test → button `on_tap`.

Details: [device.md](device.md).

## What this is not

- Not a booklet / `appmgrd` integration yet. A raw process that paints the
  framebuffer and grabs touch will fight the stock lock screen and home
  chrome. Treat booklet registration as the proper long-term host for apps.
- Not multi-threaded. Keep UI work on the loop that owns the `App`.
- The C ABI is a subset (signals, labels, buttons, stacks, app). `slot` /
  `for_each` and custom components stay Rust-only for now — see
  [c-abi.md](c-abi.md).
