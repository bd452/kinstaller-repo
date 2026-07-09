//! Stack container widget — the layout primitive behind `VStack`/`HStack`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::geometry::Size;
use crate::render::{Color, DrawCmd};
use crate::widget::{Align, AnyWidget, Axis, Common};

pub(crate) struct StackState {
    pub common: Common,
    pub axis: Axis,
    pub spacing: i32,
    pub padding: i32,
    pub align: Align,
    /// Optional solid background, painted before children. The root stack sets
    /// this so a frame can clear the panel.
    pub bg: Option<Color>,
    pub children: Vec<AnyWidget>,
}

impl StackState {
    pub(crate) fn paint(&self, out: &mut Vec<DrawCmd>) {
        if let Some(bg) = self.bg {
            out.push(DrawCmd::FillRect {
                rect: self.common.frame,
                color: bg,
            });
        }
    }
}

/// Natural size of a stack within `avail`: children summed along the main axis
/// (with spacing) and the max along the cross axis, plus padding.
pub(crate) fn measure(s: &StackState, avail: Size) -> Size {
    let pad = s.padding;
    let inner = Size::new((avail.w - 2 * pad).max(0), (avail.h - 2 * pad).max(0));
    let n = s.children.len();
    let mut main = 0;
    let mut cross = 0;
    for (i, child) in s.children.iter().enumerate() {
        let m = child.measure(inner);
        let (cm, cc) = split(s.axis, m);
        main += cm;
        cross = cross.max(cc);
        if i + 1 < n {
            main += s.spacing;
        }
    }
    join(s.axis, main + 2 * pad, cross + 2 * pad)
}

/// (main, cross) components of a size for the given axis.
pub(crate) fn split(axis: Axis, s: Size) -> (i32, i32) {
    match axis {
        Axis::Vertical => (s.h, s.w),
        Axis::Horizontal => (s.w, s.h),
    }
}

/// Rebuilds a size from (main, cross) components for the given axis.
pub(crate) fn join(axis: Axis, main: i32, cross: i32) -> Size {
    match axis {
        Axis::Vertical => Size::new(cross, main),
        Axis::Horizontal => Size::new(main, cross),
    }
}

/// A stack of widgets arranged along one axis. Cheap to clone (shares state).
#[derive(Clone)]
pub struct Stack(pub(crate) Rc<RefCell<StackState>>);

impl Stack {
    fn make(axis: Axis, spacing: i32, align: Align) -> Self {
        Stack(Rc::new(RefCell::new(StackState {
            common: Common::new(),
            axis,
            spacing,
            padding: 0,
            align,
            bg: None,
            children: Vec::new(),
        })))
    }

    /// A vertical stack.
    pub fn vertical(spacing: i32, align: Align) -> Self {
        Self::make(Axis::Vertical, spacing, align)
    }

    /// A horizontal stack.
    pub fn horizontal(spacing: i32, align: Align) -> Self {
        Self::make(Axis::Horizontal, spacing, align)
    }

    /// Uniform padding inside the stack (builder style).
    pub fn padding(self, pad: i32) -> Self {
        self.0.borrow_mut().padding = pad.max(0);
        self
    }

    /// Solid background color (builder style).
    pub fn background(self, color: Color) -> Self {
        self.0.borrow_mut().bg = Some(color);
        self
    }

    /// Appends a child.
    pub fn push(&self, child: AnyWidget) {
        let mut s = self.0.borrow_mut();
        s.children.push(child);
        s.common.dirty = true;
    }

    pub fn axis(&self) -> Axis {
        self.0.borrow().axis
    }

    // --- structural mutation used by Slot / ForEach ---

    /// Replaces the children with `ordered` (Slot swaps to one child; ForEach
    /// syncs the full list order). Marks the stack dirty.
    pub(crate) fn set_child_order(&self, ordered: &[AnyWidget]) {
        let mut s = self.0.borrow_mut();
        s.children = ordered.to_vec();
        s.common.dirty = true;
    }

    pub(crate) fn into_any(self) -> AnyWidget {
        AnyWidget::Stack(self)
    }
}
