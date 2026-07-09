//! Component-owned lifetimes.
//!
//! Port of `Lifecycle/LifecycleScope.swift`. A [`LifecycleScope`] owns
//! everything a mounted component created: its observer [`Disposable`]s, its
//! child components, and any cleanup closures. Disposing the scope tears them
//! down in the same order as the Swift original (disposables → children →
//! cleanups). In Rust ownership does most of the work — dropping the scope
//! drops the disposables (which removes the observers) — but the explicit order
//! matters when a child's teardown must run before a parent cleanup.

use crate::component::Mounted;
use crate::disposable::Disposable;

/// Owns the disposables, child components, and cleanup closures of one mounted
/// component. Created by [`crate::component::mount_component`].
#[derive(Default)]
pub struct LifecycleScope {
    alive: bool,
    disposables: Vec<Disposable>,
    children: Vec<Mounted>,
    cleanups: Vec<Box<dyn FnOnce()>>,
}

impl LifecycleScope {
    pub(crate) fn new() -> Self {
        LifecycleScope {
            alive: true,
            disposables: Vec::new(),
            children: Vec::new(),
            cleanups: Vec::new(),
        }
    }

    /// Retains a disposable for the scope's lifetime. Dropped (disposed) on
    /// [`dispose`](Self::dispose). If the scope is already dead the disposable
    /// is disposed immediately, matching the Swift guard.
    pub(crate) fn track(&mut self, mut disposable: Disposable) {
        if !self.alive {
            disposable.dispose();
            return;
        }
        self.disposables.push(disposable);
    }

    /// Adopts a mounted child component; it is unmounted when this scope
    /// disposes.
    pub(crate) fn track_child(&mut self, child: Mounted) {
        if !self.alive {
            child.unmount();
            return;
        }
        self.children.push(child);
    }

    /// Registers a closure to run during teardown.
    pub(crate) fn on_cleanup(&mut self, cleanup: impl FnOnce() + 'static) {
        if !self.alive {
            return;
        }
        self.cleanups.push(Box::new(cleanup));
    }

    /// Tears everything down, once. Order: dispose observers, unmount children,
    /// then run cleanups.
    pub(crate) fn dispose(&mut self) {
        if !self.alive {
            return;
        }
        self.alive = false;

        let disposables = std::mem::take(&mut self.disposables);
        let children = std::mem::take(&mut self.children);
        let cleanups = std::mem::take(&mut self.cleanups);

        drop(disposables); // Disposable::drop runs each teardown action.
        for child in children {
            child.unmount();
        }
        for cleanup in cleanups {
            cleanup();
        }
    }
}

impl Drop for LifecycleScope {
    fn drop(&mut self) {
        self.dispose();
    }
}
