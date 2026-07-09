//! The retained widget tree.
//!
//! SignalKit builds a `UIView`/`NSView` hierarchy; the Kindle port has none, so
//! this module supplies the widgets themselves. Each widget is a cheap-to-clone
//! `Rc<RefCell<_>>` handle (so bindings can hold and mutate it after `build()`),
//! carries a computed [`frame`](Common::frame) filled in by [`crate::layout`],
//! and a `dirty` flag set by its property setters — the signal that a repaint is
//! needed, which is how signal-driven mutation maps onto minimal e-ink redraws.

use std::cell::Cell;

use crate::geometry::{Point, Rect, Size};
use crate::render::DrawCmd;

pub mod button;
pub mod label;
pub mod spacer;
pub mod stack;

pub use button::Button;
pub use label::Label;
pub use spacer::Spacer;
pub use stack::Stack;

/// Font cell metrics for the FBInk built-in font at size multiplier 1. Layout
/// and the FBInk renderer share these so measured rectangles line up with what
/// is actually drawn. (FBInk's default IBM VGA font is 8x16 at 1x.)
///
/// The on-device renderer must `fbink_init` with a matching `fontmult` before
/// each print — FBInk ignores `fontmult` on the print call itself.
pub mod font {
    pub const CELL_W: i32 = 8;
    pub const CELL_H: i32 = 16;

    /// Pixel size of `text` at the given size multiplier, single line.
    pub fn text_size(text: &str, size: u8) -> super::Size {
        let cols = text.chars().count() as i32;
        super::Size::new(cols * CELL_W * size as i32, CELL_H * size as i32)
    }
}

/// A process-unique widget identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetId(pub u64);

thread_local! {
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
}

impl WidgetId {
    pub(crate) fn next() -> WidgetId {
        NEXT_ID.with(|c| {
            let id = c.get();
            c.set(id + 1);
            WidgetId(id)
        })
    }
}

/// Fields every widget carries. Embedded in each widget's state struct.
#[derive(Debug)]
pub struct Common {
    pub id: WidgetId,
    /// Absolute screen rectangle, written by layout.
    pub frame: Rect,
    /// Set by property setters; cleared after a repaint.
    pub dirty: bool,
}

impl Common {
    pub(crate) fn new() -> Self {
        Common {
            id: WidgetId::next(),
            frame: Rect::ZERO,
            dirty: true,
        }
    }
}

/// Cross-axis alignment of stack children. A reduced form of SignalKit's
/// `StackAlignment` (the baseline variants have no meaning without a text
/// layout engine).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Fill,
    Leading,
    Center,
    Trailing,
}

/// Stack orientation. Port of `StackAxis`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// A node in the widget tree. Each variant holds a cloneable handle, so cloning
/// an `AnyWidget` shares the same underlying widget.
#[derive(Clone)]
pub enum AnyWidget {
    Label(Label),
    Button(Button),
    Stack(Stack),
    Spacer(Spacer),
}

impl AnyWidget {
    pub fn id(&self) -> WidgetId {
        match self {
            AnyWidget::Label(w) => w.0.borrow().common.id,
            AnyWidget::Button(w) => w.0.borrow().common.id,
            AnyWidget::Stack(w) => w.0.borrow().common.id,
            AnyWidget::Spacer(w) => w.0.borrow().common.id,
        }
    }

    pub fn frame(&self) -> Rect {
        match self {
            AnyWidget::Label(w) => w.0.borrow().common.frame,
            AnyWidget::Button(w) => w.0.borrow().common.frame,
            AnyWidget::Stack(w) => w.0.borrow().common.frame,
            AnyWidget::Spacer(w) => w.0.borrow().common.frame,
        }
    }

    pub(crate) fn set_frame(&self, r: Rect) {
        match self {
            AnyWidget::Label(w) => w.0.borrow_mut().common.frame = r,
            AnyWidget::Button(w) => w.0.borrow_mut().common.frame = r,
            AnyWidget::Stack(w) => w.0.borrow_mut().common.frame = r,
            AnyWidget::Spacer(w) => w.0.borrow_mut().common.frame = r,
        }
    }

    pub fn is_dirty(&self) -> bool {
        match self {
            AnyWidget::Label(w) => w.0.borrow().common.dirty,
            AnyWidget::Button(w) => w.0.borrow().common.dirty,
            AnyWidget::Stack(w) => w.0.borrow().common.dirty,
            AnyWidget::Spacer(w) => w.0.borrow().common.dirty,
        }
    }

    pub(crate) fn clear_dirty(&self) {
        match self {
            AnyWidget::Label(w) => w.0.borrow_mut().common.dirty = false,
            AnyWidget::Button(w) => w.0.borrow_mut().common.dirty = false,
            AnyWidget::Stack(w) => w.0.borrow_mut().common.dirty = false,
            AnyWidget::Spacer(w) => w.0.borrow_mut().common.dirty = false,
        }
    }

    /// Natural size within `avail`, ignoring the widget's current frame. Used by
    /// the layout solver.
    pub fn measure(&self, avail: Size) -> Size {
        match self {
            AnyWidget::Label(w) => w.0.borrow().measure(),
            AnyWidget::Button(w) => w.0.borrow().measure(),
            AnyWidget::Spacer(w) => w.0.borrow().measure(),
            AnyWidget::Stack(w) => stack::measure(&w.0.borrow(), avail),
        }
    }

    /// A flexible widget (a [`Spacer`]) absorbs leftover main-axis space.
    pub fn is_flexible(&self) -> bool {
        matches!(self, AnyWidget::Spacer(_))
    }

    /// Direct children (only stacks have any).
    pub fn children(&self) -> Vec<AnyWidget> {
        match self {
            AnyWidget::Stack(w) => w.0.borrow().children.clone(),
            _ => Vec::new(),
        }
    }

    /// Appends this widget's own draw commands (not its children's) to `out`.
    pub fn paint_self(&self, out: &mut Vec<DrawCmd>) {
        match self {
            AnyWidget::Label(w) => w.0.borrow().paint(out),
            AnyWidget::Button(w) => w.0.borrow().paint(out),
            AnyWidget::Spacer(_) => {}
            AnyWidget::Stack(w) => w.0.borrow().paint(out),
        }
    }

    /// Invokes the tap handler if this widget is an interactive one. Returns
    /// true if a handler ran.
    pub fn dispatch_tap(&self) -> bool {
        if let AnyWidget::Button(w) = self {
            return w.fire_tap();
        }
        false
    }
}

/// Walks `root` and every descendant, calling `f` on each (pre-order).
pub fn walk(root: &AnyWidget, f: &mut impl FnMut(&AnyWidget)) {
    f(root);
    for child in root.children() {
        walk(&child, f);
    }
}

/// Returns the topmost interactive (tappable) widget whose frame contains `p`,
/// searching children before parents so nested content wins. Used by hit
/// testing.
pub fn hit_test(root: &AnyWidget, p: Point) -> Option<AnyWidget> {
    if !root.frame().contains(p) {
        return None;
    }
    // Children are drawn/placed after the parent, so a child at `p` is on top.
    for child in root.children().iter().rev() {
        if let Some(hit) = hit_test(child, p) {
            return Some(hit);
        }
    }
    if matches!(root, AnyWidget::Button(_)) {
        return Some(root.clone());
    }
    None
}
