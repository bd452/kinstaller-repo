//! One-shot teardown handles.
//!
//! Port of `Core/Disposable.swift`. Swift models a `Disposable` as a class with
//! a `dispose()` method that is safe to call repeatedly. In Rust we get the
//! same lifetime semantics for free from ownership: a [`Disposable`] runs its
//! action on explicit [`Disposable::dispose`] *or* when dropped, whichever
//! comes first, and never twice.

/// Runs a teardown action exactly once, on `dispose()` or on drop.
///
/// Returned by [`crate::Signal::observe`] and stored by [`crate::LifecycleScope`].
/// Dropping the handle disposes it, so a subscription lives exactly as long as
/// the scope (or binding) that owns its handle — this is how unmounting a
/// component tears down its observers.
#[must_use = "dropping a Disposable immediately tears down the subscription; \
              store it (e.g. via a scope) to keep it alive"]
pub struct Disposable {
    action: Option<Box<dyn FnOnce()>>,
}

impl Disposable {
    /// Wraps a teardown closure.
    pub fn new(action: impl FnOnce() + 'static) -> Self {
        Self {
            action: Some(Box::new(action)),
        }
    }

    /// A disposable that does nothing.
    pub fn noop() -> Self {
        Self { action: None }
    }

    /// Runs the teardown action now, if it hasn't run already.
    pub fn dispose(&mut self) {
        if let Some(action) = self.action.take() {
            action();
        }
    }

    /// Drops the handle *without* running the action, intentionally leaking the
    /// subscription for the lifetime of the underlying signal. Mirrors the
    /// escape hatch of not retaining a returned `Disposable` in Swift.
    pub fn forget(mut self) {
        self.action = None;
    }
}

impl Drop for Disposable {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn runs_on_explicit_dispose() {
        let hit = Rc::new(Cell::new(0));
        let h = hit.clone();
        let mut d = Disposable::new(move || h.set(h.get() + 1));
        d.dispose();
        assert_eq!(hit.get(), 1);
    }

    #[test]
    fn runs_at_most_once() {
        let hit = Rc::new(Cell::new(0));
        let h = hit.clone();
        let mut d = Disposable::new(move || h.set(h.get() + 1));
        d.dispose();
        d.dispose();
        drop(d);
        assert_eq!(hit.get(), 1);
    }

    #[test]
    fn runs_on_drop() {
        let hit = Rc::new(Cell::new(0));
        let h = hit.clone();
        let d = Disposable::new(move || h.set(h.get() + 1));
        drop(d);
        assert_eq!(hit.get(), 1);
    }

    #[test]
    fn forget_skips_action() {
        let hit = Rc::new(Cell::new(0));
        let h = hit.clone();
        Disposable::new(move || h.set(h.get() + 1)).forget();
        assert_eq!(hit.get(), 0);
    }
}
