//! Components and mounting.
//!
//! Port of `Component/Component.swift`. Swift uses an open class with an
//! overridable `build()`; Rust has no inheritance, so [`Component`] is a trait
//! and the services the Swift base class offered (`bind`, `observe`, `track`,
//! `host`) are provided by [`BuildCtx`], passed into `build`. `build` runs once;
//! afterwards a signal firing mutates widget properties directly through the
//! closures registered here — there is no re-render or diffing.

use crate::disposable::Disposable;
use crate::lifecycle::LifecycleScope;
use crate::node::Node;
use crate::signal::Signal;
use crate::widget::{Align, AnyWidget, Axis, Stack};

/// A unit of UI. Implement `build` to construct the subtree; override the
/// lifecycle hooks as needed.
pub trait Component {
    /// Builds the subtree once, at mount. Register bindings/observers via `ctx`.
    fn build(&mut self, ctx: &mut BuildCtx) -> Node;

    /// Called after the subtree is mounted.
    fn did_mount(&mut self) {}

    /// Called before teardown.
    fn will_unmount(&mut self) {}
}

/// A mounted component: the component itself, its lifetime scope, and the root
/// widget it produced.
pub struct Mounted {
    component: Box<dyn Component>,
    scope: LifecycleScope,
    root: AnyWidget,
}

impl Mounted {
    /// The root widget of the mounted subtree.
    pub fn root(&self) -> AnyWidget {
        self.root.clone()
    }

    /// Tears the component down: `will_unmount`, then dispose the scope (which
    /// unmounts children and removes observers).
    pub(crate) fn unmount(mut self) {
        self.component.will_unmount();
        self.scope.dispose();
    }
}

/// Services available during [`Component::build`]. Everything registered here
/// is owned by the component's scope and torn down on unmount.
pub struct BuildCtx<'a> {
    scope: &'a mut LifecycleScope,
}

impl BuildCtx<'_> {
    /// Retains a disposable for the component's lifetime.
    pub fn track(&mut self, disposable: Disposable) {
        self.scope.track(disposable);
    }

    /// Registers a cleanup closure run at unmount.
    pub fn on_cleanup(&mut self, cleanup: impl FnOnce() + 'static) {
        self.scope.on_cleanup(cleanup);
    }

    /// Observes `signal`. When `fire_immediately`, `handler` runs once now with
    /// the current value. The subscription lives until the component unmounts.
    pub fn observe<T: 'static>(
        &mut self,
        signal: &Signal<T>,
        fire_immediately: bool,
        mut handler: impl FnMut(&T) + 'static,
    ) {
        if fire_immediately {
            signal.with(|v| handler(v));
        }
        let disposable = signal.observe(handler);
        self.scope.track(disposable);
    }

    /// Binds `signal` to `apply`: runs `apply` with the current value now and on
    /// every change. Replaces SignalKit's keypath `bind` — `apply` is the setter
    /// closure, typically mutating a widget handle it captures.
    pub fn bind<T: 'static>(&mut self, signal: &Signal<T>, apply: impl FnMut(&T) + 'static) {
        self.observe(signal, true, apply);
    }

    /// Mounts `child` and returns its root widget; the child's lifetime is tied
    /// to this component. Port of `host(_:)`.
    pub fn host(&mut self, child: Box<dyn Component>) -> AnyWidget {
        let mounted = mount_component(child);
        let root = mounted.root.clone();
        self.scope.track_child(mounted);
        root
    }
}

/// Mounts a component: creates its scope, runs `build`, materializes the node
/// tree into widgets, then calls `did_mount`.
pub fn mount_component(mut component: Box<dyn Component>) -> Mounted {
    let mut scope = LifecycleScope::new();
    let node = {
        let mut ctx = BuildCtx { scope: &mut scope };
        component.build(&mut ctx)
    };
    let root = mount_node(node, &mut scope);
    component.did_mount();
    Mounted {
        component,
        scope,
        root,
    }
}

/// Materializes a [`Node`] into a widget, mounting any child components into
/// `scope`.
pub fn mount_node(node: Node, scope: &mut LifecycleScope) -> AnyWidget {
    match node {
        Node::Widget(w) => w,
        Node::Stack {
            axis,
            spacing,
            padding,
            align,
            bg,
            children,
        } => {
            let mut stack = match axis {
                Axis::Vertical => Stack::vertical(spacing, align),
                Axis::Horizontal => Stack::horizontal(spacing, align),
            };
            if padding > 0 {
                stack = stack.padding(padding);
            }
            if let Some(color) = bg {
                stack = stack.background(color);
            }
            for child in children {
                let child_widget = mount_node(child, scope);
                stack.push(child_widget);
            }
            stack.into_any()
        }
        Node::Component(boxed) => {
            let mounted = mount_component(boxed);
            let root = mounted.root.clone();
            scope.track_child(mounted);
            root
        }
        Node::Group(_) => panic!(
            "a bare Group cannot be mounted as a subtree root; wrap children in vstack()/hstack()"
        ),
    }
}

/// A stack container that a structural component (Slot/ForEach) mutates after
/// mount. Exposed so those components share the same container type the layout
/// solver understands.
pub(crate) fn container(axis: Axis) -> Stack {
    match axis {
        Axis::Vertical => Stack::vertical(0, Align::Fill),
        Axis::Horizontal => Stack::horizontal(0, Align::Fill),
    }
}

/// Wraps an already-built [`Node`] as a trivial component. Lets `Slot`/`ForEach`
/// content closures return a `Node` directly; if such content needs its own
/// reactive bindings it embeds a `Node::Component` with a real `Component`.
struct NodeComponent(Option<Node>);

impl Component for NodeComponent {
    fn build(&mut self, _ctx: &mut BuildCtx) -> Node {
        self.0.take().expect("NodeComponent built more than once")
    }
}

/// Boxes a [`Node`] as a [`Component`] (see [`NodeComponent`]).
pub(crate) fn node_component(node: Node) -> Box<dyn Component> {
    Box::new(NodeComponent(Some(node)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{vstack, IntoNode};
    use crate::widget::{Button, Label};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Counter {
        count: Signal<i32>,
    }

    impl Component for Counter {
        fn build(&mut self, ctx: &mut BuildCtx) -> Node {
            let label = Label::new("");
            let bind_label = label.clone();
            ctx.bind(&self.count, move |v| bind_label.set_text(format!("Count: {v}")));

            let inc = self.count.clone();
            let button = Button::new("+").on_tap(move || inc.update(|v| v + 1));

            vstack(
                8,
                Align::Fill,
                vec![label.into_node(), button.into_node()],
            )
        }
    }

    #[test]
    fn binding_applies_current_value_at_mount() {
        let count = Signal::new(41);
        let mounted = mount_component(Box::new(Counter { count: count.clone() }));
        // The bound label reflects the signal's value immediately.
        let root = mounted.root();
        let label = &root.children()[0];
        if let AnyWidget::Label(l) = label {
            assert_eq!(l.text(), "Count: 41");
        } else {
            panic!("expected label");
        }
    }

    #[test]
    fn signal_change_mutates_widget_after_mount() {
        let count = Signal::new(0);
        let mounted = mount_component(Box::new(Counter { count: count.clone() }));
        count.set(7);
        let root = mounted.root();
        if let AnyWidget::Label(l) = &root.children()[0] {
            assert_eq!(l.text(), "Count: 7");
            assert!(l.0.borrow().common.dirty, "changed label marked dirty");
        }
    }

    #[test]
    fn button_tap_runs_handler() {
        let count = Signal::new(0);
        let mounted = mount_component(Box::new(Counter { count: count.clone() }));
        let root = mounted.root();
        let tapped = root.children()[1].dispatch_tap();
        assert!(tapped);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn unmount_stops_observers() {
        let count = Signal::new(0);
        let fired = Rc::new(RefCell::new(0));

        struct Probe {
            sig: Signal<i32>,
            fired: Rc<RefCell<i32>>,
        }
        impl Component for Probe {
            fn build(&mut self, ctx: &mut BuildCtx) -> Node {
                let fired = self.fired.clone();
                ctx.observe(&self.sig, false, move |_| *fired.borrow_mut() += 1);
                Label::new("x").into_node()
            }
        }

        let mounted = mount_component(Box::new(Probe {
            sig: count.clone(),
            fired: fired.clone(),
        }));
        count.set(1);
        assert_eq!(*fired.borrow(), 1);
        mounted.unmount();
        count.set(2);
        assert_eq!(*fired.borrow(), 1, "observer removed on unmount");
    }
}
