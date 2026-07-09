//! Dynamic single-child replacement. Port of `SlotComponent`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::component::{container, mount_component, node_component, BuildCtx, Component, Mounted};
use crate::node::{IntoNode, Node};
use crate::signal::Signal;
use crate::widget::{Axis, Stack};

/// Mounts the component produced by `content(value)` and re-mounts it whenever
/// `signal` changes, swapping it inside a stable container. Port of `Slot`.
///
/// `content` returns a [`Node`]; for per-value reactive content, embed a
/// `Node::Component` inside it.
pub fn slot<T: 'static>(signal: &Signal<T>, content: impl Fn(&T) -> Node + 'static) -> Node {
    Node::Component(Box::new(SlotComponent {
        signal: signal.clone(),
        content: Rc::new(content),
        container: container(Axis::Vertical),
        current: Rc::new(RefCell::new(None)),
    })
    .into_boxed())
}

struct SlotComponent<T> {
    signal: Signal<T>,
    content: Rc<dyn Fn(&T) -> Node>,
    container: Stack,
    current: Rc<RefCell<Option<Mounted>>>,
}

impl<T: 'static> SlotComponent<T> {
    fn into_boxed(self) -> Box<dyn Component> {
        Box::new(self)
    }
}

/// Mounts `node` into `container` as its sole child, unmounting whatever was
/// there before.
fn mount_into(container: &Stack, current: &Rc<RefCell<Option<Mounted>>>, node: Node) {
    if let Some(old) = current.borrow_mut().take() {
        old.unmount();
    }
    let mounted = mount_component(node_component(node));
    container.set_child_order(&[mounted.root()]);
    *current.borrow_mut() = Some(mounted);
}

impl<T: 'static> Component for SlotComponent<T> {
    fn build(&mut self, ctx: &mut BuildCtx) -> Node {
        let initial = self.signal.with(|v| (self.content)(v));
        mount_into(&self.container, &self.current, initial);

        let container = self.container.clone();
        let content = self.content.clone();
        let current = self.current.clone();
        ctx.observe(&self.signal, false, move |value| {
            let node = content(value);
            mount_into(&container, &current, node);
        });

        self.container.clone().into_node()
    }

    fn will_unmount(&mut self) {
        if let Some(old) = self.current.borrow_mut().take() {
            old.unmount();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::mount_component;
    use crate::node::IntoNode;
    use crate::signal::Signal;
    use crate::widget::{AnyWidget, Label};

    fn root_text(root: &AnyWidget) -> String {
        // container -> single child label
        if let AnyWidget::Label(l) = &root.children()[0] {
            l.text()
        } else {
            panic!("expected label child");
        }
    }

    #[test]
    fn swaps_child_on_signal_change() {
        let which = Signal::new(0);
        let node = slot(&which, |v| Label::new(format!("v{v}")).into_node());
        let mounted = mount_component(node_component(node));
        let root = mounted.root();
        assert_eq!(root_text(&root), "v0");
        which.set(5);
        assert_eq!(root_text(&root), "v5");
        // Still exactly one child after the swap.
        assert_eq!(root.children().len(), 1);
    }
}
