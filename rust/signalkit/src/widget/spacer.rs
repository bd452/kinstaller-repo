//! Flexible spacer widget — absorbs leftover main-axis space in a stack.

use std::cell::RefCell;
use std::rc::Rc;

use crate::geometry::Size;
use crate::widget::Common;

pub(crate) struct SpacerState {
    pub common: Common,
    /// Minimum extent along the stack's main axis.
    pub min: i32,
}

impl SpacerState {
    pub(crate) fn measure(&self) -> Size {
        Size::new(self.min, self.min)
    }
}

/// A flexible gap. In a stack it takes zero natural size but expands to consume
/// leftover space, so `[A, Spacer, B]` pushes A and B to opposite ends.
#[derive(Clone)]
pub struct Spacer(pub(crate) Rc<RefCell<SpacerState>>);

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

impl Spacer {
    pub fn new() -> Self {
        Spacer(Rc::new(RefCell::new(SpacerState {
            common: Common::new(),
            min: 0,
        })))
    }

    /// A spacer that is at least `min` pixels along the main axis.
    pub fn min(min: i32) -> Self {
        let s = Self::new();
        s.0.borrow_mut().min = min.max(0);
        s
    }
}
