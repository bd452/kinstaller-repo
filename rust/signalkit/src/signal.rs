//! Fine-grained observable state.
//!
//! Port of `Signal/Signal.swift`. A [`Signal`] is a cloneable handle
//! (`Rc<RefCell<..>>`) to a shared value plus a set of observers. Writing
//! notifies observers synchronously; the write-coalescing behaviour of the
//! Swift original is preserved: a write made *from inside* an observer callback
//! does not re-enter delivery, it schedules a single follow-up pass with the
//! final value.
//!
//! Everything here is single-threaded (`!Send`/`!Sync` via `Rc`), matching the
//! `@MainActor` isolation of the source. The on-device event loop is the
//! moral equivalent of the main actor.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

use crate::disposable::Disposable;

/// One registered observer. Held behind an `Rc` so delivery can take a cheap
/// snapshot of the observer set and call handlers *without* holding a borrow on
/// the signal — handlers routinely re-enter [`Signal::set`] and may dispose
/// themselves or other observers mid-delivery.
struct ObserverSlot<T> {
    handler: RefCell<Option<Box<dyn FnMut(&T)>>>,
    disposed: Cell<bool>,
}

struct Inner<T> {
    value: T,
    observers: BTreeMap<u64, Rc<ObserverSlot<T>>>,
    next_id: u64,
    is_notifying: bool,
    has_pending: bool,
}

/// An observable value. Cheap to [`Clone`] (shares the same underlying state).
pub struct Signal<T> {
    inner: Rc<RefCell<Inner<T>>>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Signal {
            inner: self.inner.clone(),
        }
    }
}

impl<T: 'static> Signal<T> {
    /// Creates a signal holding `value`.
    pub fn new(value: T) -> Self {
        Signal {
            inner: Rc::new(RefCell::new(Inner {
                value,
                observers: BTreeMap::new(),
                next_id: 0,
                is_notifying: false,
                has_pending: false,
            })),
        }
    }

    /// Reads the current value via a closure, avoiding a `Clone` bound.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.inner.borrow().value)
    }

    /// Registers `handler`, called on every subsequent change (not immediately —
    /// mirrors `Signal.observe`; [`crate::component::BuildCtx::observe`] adds the
    /// fire-immediately behaviour). The returned [`Disposable`] removes the
    /// observer when disposed or dropped.
    #[must_use]
    pub fn observe(&self, handler: impl FnMut(&T) + 'static) -> Disposable {
        let id = {
            let mut inner = self.inner.borrow_mut();
            let id = inner.next_id;
            inner.next_id = inner.next_id.wrapping_add(1);
            inner.observers.insert(
                id,
                Rc::new(ObserverSlot {
                    handler: RefCell::new(Some(Box::new(handler))),
                    disposed: Cell::new(false),
                }),
            );
            id
        };

        let weak: Weak<RefCell<Inner<T>>> = Rc::downgrade(&self.inner);
        Disposable::new(move || {
            if let Some(inner) = weak.upgrade() {
                if let Some(slot) = inner.borrow_mut().observers.remove(&id) {
                    slot.disposed.set(true);
                    slot.handler.borrow_mut().take();
                }
            }
        })
    }
}

impl<T: Clone + 'static> Signal<T> {
    /// Returns a clone of the current value.
    pub fn get(&self) -> T {
        self.inner.borrow().value.clone()
    }

    fn deliver(&self) {
        loop {
            // Clone the value and snapshot the observer set (cheap Rc clones),
            // releasing every borrow *before* invoking any handler — handlers
            // routinely re-enter `set`/`dispose`, which need `borrow_mut`.
            let value = self.inner.borrow().value.clone();
            let snapshot: Vec<Rc<ObserverSlot<T>>> = {
                let inner = self.inner.borrow();
                inner.observers.values().cloned().collect()
            };

            for slot in &snapshot {
                if slot.disposed.get() {
                    continue;
                }
                // Take the closure out so the slot's RefCell is not borrowed
                // during the call: the handler may dispose *this* observer
                // (which touches the same slot) without a borrow conflict.
                let taken = slot.handler.borrow_mut().take();
                if let Some(mut handler) = taken {
                    handler(&value);
                    // Restore only if it wasn't disposed during the callback.
                    if !slot.disposed.get() {
                        *slot.handler.borrow_mut() = Some(handler);
                    }
                }
            }

            let mut inner = self.inner.borrow_mut();
            if inner.has_pending {
                // A write occurred during delivery. `value` is already the
                // latest; run one more pass with it and clear the flag.
                inner.has_pending = false;
                drop(inner);
                continue;
            }
            inner.is_notifying = false;
            break;
        }
    }

    /// Sets a new value and notifies observers. A `set` made from inside an
    /// observer callback is coalesced into a single follow-up delivery.
    pub fn set(&self, value: T) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.value = value;
            if inner.is_notifying {
                inner.has_pending = true;
                return;
            }
            inner.is_notifying = true;
        }
        self.deliver();
    }

    /// Replaces the value with `f(current)` and notifies.
    pub fn update(&self, f: impl FnOnce(&T) -> T) {
        let next = self.with(|v| f(v));
        self.set(next);
    }
}

impl<T: Clone + PartialEq + 'static> Signal<T> {
    /// Sets a new value only if it differs from the current one.
    pub fn set_if_changed(&self, value: T) {
        if self.with(|current| *current != value) {
            self.set(value);
        }
    }

    /// Applies `f` and sets the result only if it differs.
    pub fn update_if_changed(&self, f: impl FnOnce(&T) -> T) {
        let next = self.with(|v| f(v));
        self.set_if_changed(next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn observers_fire_on_change_not_on_register() {
        let s = Signal::new(1);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        let _d = s.observe(move |v: &i32| sink.borrow_mut().push(*v));
        assert!(seen.borrow().is_empty(), "observe must not fire immediately");
        s.set(2);
        s.set(3);
        assert_eq!(*seen.borrow(), vec![2, 3]);
    }

    #[test]
    fn disposing_stops_delivery_and_removes_observer() {
        let s = Signal::new(0);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        let mut d = s.observe(move |v: &i32| sink.borrow_mut().push(*v));
        s.set(1);
        d.dispose();
        s.set(2);
        assert_eq!(*seen.borrow(), vec![1]);
    }

    #[test]
    fn nested_writes_coalesce_to_one_followup() {
        // An observer that writes back to the signal must not re-enter
        // delivery; it schedules a single follow-up with the final value.
        let s = Signal::new(0);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let s2 = s.clone();
        let sink = seen.clone();
        let _d = s.observe(move |v: &i32| {
            sink.borrow_mut().push(*v);
            if *v < 3 {
                // Coalesced: only the final value (3) is redelivered.
                s2.set(*v + 10);
                s2.set(3);
            }
        });
        s.set(1);
        // First pass sees 1; follow-up pass sees 3; the intermediate 13 is
        // coalesced away exactly like the Swift original.
        assert_eq!(*seen.borrow(), vec![1, 3]);
        assert_eq!(s.get(), 3);
    }

    #[test]
    fn observer_can_dispose_itself_during_delivery() {
        let s = Signal::new(0);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let slot: Rc<RefCell<Option<Disposable>>> = Rc::new(RefCell::new(None));
        let sink = seen.clone();
        let slot2 = slot.clone();
        let d = s.observe(move |v: &i32| {
            sink.borrow_mut().push(*v);
            // Dispose self mid-callback — must not panic on a re-borrow.
            slot2.borrow_mut().take();
        });
        *slot.borrow_mut() = Some(d);
        s.set(1);
        s.set(2);
        assert_eq!(*seen.borrow(), vec![1], "self-disposed after first fire");
    }

    #[test]
    fn set_if_changed_suppresses_equal_writes() {
        let s = Signal::new(5);
        let count = Rc::new(RefCell::new(0));
        let c = count.clone();
        let _d = s.observe(move |_: &i32| *c.borrow_mut() += 1);
        s.set_if_changed(5);
        s.set_if_changed(6);
        s.set_if_changed(6);
        assert_eq!(*count.borrow(), 1);
    }
}
