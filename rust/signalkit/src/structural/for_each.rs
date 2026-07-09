//! Keyed dynamic collection. Port of `ForEachComponent`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

use crate::component::{container, mount_component, node_component, BuildCtx, Component, Mounted};
use crate::node::{IntoNode, Node};
use crate::signal::Signal;
use crate::widget::{Axis, Stack};

/// Renders one child per element of a `Vec<T>` signal, identified by `key`, and
/// keeps them in sync as the collection changes: children whose key disappears
/// are unmounted, new keys are mounted, and the container order follows the
/// data. Duplicate keys panic (mirrors the Swift precondition). Port of
/// `ForEach`.
pub fn for_each<T, K>(
    signal: &Signal<Vec<T>>,
    key: impl Fn(&T) -> K + 'static,
    content: impl Fn(&T) -> Node + 'static,
) -> Node
where
    T: Clone + 'static,
    K: Hash + Eq + Clone + std::fmt::Debug + 'static,
{
    Node::Component(Box::new(ForEachComponent {
        signal: signal.clone(),
        key: Rc::new(key),
        content: Rc::new(content),
        container: container(Axis::Vertical),
        children: Rc::new(RefCell::new(HashMap::new())),
    }))
}

struct ForEachComponent<T, K> {
    signal: Signal<Vec<T>>,
    key: Rc<dyn Fn(&T) -> K>,
    content: Rc<dyn Fn(&T) -> Node>,
    container: Stack,
    children: Rc<RefCell<HashMap<K, Mounted>>>,
}

/// Reconciles the mounted children against `data`.
fn apply<T, K>(
    data: &[T],
    key: &Rc<dyn Fn(&T) -> K>,
    content: &Rc<dyn Fn(&T) -> Node>,
    container: &Stack,
    children: &Rc<RefCell<HashMap<K, Mounted>>>,
) where
    T: Clone,
    K: Hash + Eq + Clone + std::fmt::Debug,
{
    let ordered_keys: Vec<K> = data.iter().map(|e| key(e)).collect();

    // Duplicate-key check (Swift precondition).
    {
        let mut seen = std::collections::HashSet::new();
        for k in &ordered_keys {
            assert!(
                seen.insert(k.clone()),
                "for_each data contains duplicate key {k:?}; each element must be uniquely identified"
            );
        }
    }

    let new_set: std::collections::HashSet<&K> = ordered_keys.iter().collect();

    // Remove children whose key vanished.
    let removed: Vec<K> = children
        .borrow()
        .keys()
        .filter(|k| !new_set.contains(k))
        .cloned()
        .collect();
    for k in removed {
        if let Some(child) = children.borrow_mut().remove(&k) {
            child.unmount();
        }
    }

    // Mount newly appeared keys.
    for element in data {
        let k = key(element);
        if children.borrow().contains_key(&k) {
            continue;
        }
        let mounted = mount_component(node_component(content(element)));
        children.borrow_mut().insert(k, mounted);
    }

    // Sync order to match the data.
    let ordered_widgets = {
        let map = children.borrow();
        ordered_keys
            .iter()
            .filter_map(|k| map.get(k).map(|m| m.root()))
            .collect::<Vec<_>>()
    };
    container.set_child_order(&ordered_widgets);
}

impl<T, K> Component for ForEachComponent<T, K>
where
    T: Clone + 'static,
    K: Hash + Eq + Clone + std::fmt::Debug + 'static,
{
    fn build(&mut self, ctx: &mut BuildCtx) -> Node {
        self.signal.with(|data| {
            apply(
                data,
                &self.key,
                &self.content,
                &self.container,
                &self.children,
            )
        });

        let key = self.key.clone();
        let content = self.content.clone();
        let container = self.container.clone();
        let children = self.children.clone();
        ctx.observe(&self.signal, false, move |data| {
            apply(data, &key, &content, &container, &children);
        });

        self.container.clone().into_node()
    }

    fn will_unmount(&mut self) {
        for (_, child) in self.children.borrow_mut().drain() {
            child.unmount();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{AnyWidget, Label};

    fn child_texts(root: &AnyWidget) -> Vec<String> {
        root.children()
            .iter()
            .map(|c| match c {
                AnyWidget::Label(l) => l.text(),
                _ => panic!("expected label"),
            })
            .collect()
    }

    #[test]
    fn mounts_reorders_and_removes_by_key() {
        let items = Signal::new(vec![1, 2, 3]);
        let node = for_each(&items, |n| *n, |n| Label::new(format!("#{n}")).into_node());
        let mounted = mount_component(node_component(node));
        let root = mounted.root();
        assert_eq!(child_texts(&root), vec!["#1", "#2", "#3"]);

        // Reorder + remove one + add one.
        items.set(vec![3, 1, 4]);
        assert_eq!(child_texts(&root), vec!["#3", "#1", "#4"]);
    }

    #[test]
    fn reused_children_keep_identity_across_reorder() {
        let items = Signal::new(vec![1, 2]);
        let node = for_each(&items, |n| *n, |n| Label::new(format!("#{n}")).into_node());
        let mounted = mount_component(node_component(node));
        let root = mounted.root();
        let id_of_1 = root.children()[0].id();

        items.set(vec![2, 1]);
        // The widget for key 1 moved but is the same instance.
        let id_of_1_after = root.children()[1].id();
        assert_eq!(id_of_1, id_of_1_after);
    }

    #[test]
    #[should_panic(expected = "duplicate key")]
    fn duplicate_keys_panic() {
        let items = Signal::new(vec![1, 1]);
        let node = for_each(&items, |n| *n, |n| Label::new(format!("#{n}")).into_node());
        let _ = mount_component(node_component(node));
    }
}
