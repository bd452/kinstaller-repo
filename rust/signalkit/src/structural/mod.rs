//! Dynamic structure: [`slot`] and [`for_each`].
//!
//! Everything else in the framework builds its view tree once. These two are the
//! only pieces that change the tree *after* mount, in response to a signal —
//! ports of `Structural/Slot.swift` and `Structural/ForEach.swift`. Both are
//! components that own a container [`Stack`](crate::widget::Stack) and mutate its
//! children directly.

mod for_each;
mod slot;

pub use for_each::for_each;
pub use slot::slot;
