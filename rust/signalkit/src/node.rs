//! Declarative composition.
//!
//! Port of `Node/Node.swift`. A [`Node`] is the value returned from
//! [`Component::build`](crate::component::Component::build); it describes the
//! subtree to construct *once* at mount. Swift uses a `@resultBuilder`; Rust
//! uses plain [`vstack`]/[`hstack`] functions plus the [`IntoNode`] conversion
//! so widgets and components drop in directly.

use crate::component::Component;
use crate::render::Color;
use crate::widget::{Align, AnyWidget, Axis, Button, Label, Spacer, Stack};

/// A description of a subtree, consumed at mount time.
pub enum Node {
    /// A concrete widget to insert as-is.
    Widget(AnyWidget),
    /// A stack container to build, with child nodes.
    Stack {
        axis: Axis,
        spacing: i32,
        padding: i32,
        align: Align,
        bg: Option<Color>,
        children: Vec<Node>,
    },
    /// A child component to mount (its lifetime tied to the parent scope).
    Component(Box<dyn Component>),
    /// A flattened group of nodes (spliced into the enclosing stack).
    Group(Vec<Node>),
}

/// Conversion into a [`Node`]. Implemented for every widget handle and for
/// boxed components so composition reads naturally.
pub trait IntoNode {
    fn into_node(self) -> Node;
}

impl IntoNode for Node {
    fn into_node(self) -> Node {
        self
    }
}

impl IntoNode for AnyWidget {
    fn into_node(self) -> Node {
        Node::Widget(self)
    }
}

macro_rules! into_node_widget {
    ($ty:ty, $variant:ident) => {
        impl IntoNode for $ty {
            fn into_node(self) -> Node {
                Node::Widget(AnyWidget::$variant(self))
            }
        }
    };
}
into_node_widget!(Label, Label);
into_node_widget!(Button, Button);
into_node_widget!(Spacer, Spacer);
into_node_widget!(Stack, Stack);

impl IntoNode for Box<dyn Component> {
    fn into_node(self) -> Node {
        Node::Component(self)
    }
}

/// Flattens [`Node::Group`]s one level, mirroring Swift's `flattenChildren`.
fn flatten(children: Vec<Node>) -> Vec<Node> {
    let mut out = Vec::with_capacity(children.len());
    for child in children {
        match child {
            Node::Group(inner) => out.extend(flatten(inner)),
            other => out.push(other),
        }
    }
    out
}

/// A vertical stack of children. Port of `VStack`.
pub fn vstack(spacing: i32, align: Align, children: Vec<Node>) -> Node {
    Node::Stack {
        axis: Axis::Vertical,
        spacing,
        padding: 0,
        align,
        bg: None,
        children: flatten(children),
    }
}

/// A horizontal stack of children. Port of `HStack`.
pub fn hstack(spacing: i32, align: Align, children: Vec<Node>) -> Node {
    Node::Stack {
        axis: Axis::Horizontal,
        spacing,
        padding: 0,
        align,
        bg: None,
        children: flatten(children),
    }
}

/// Groups several nodes so they can be returned where a single [`Node`] is
/// expected; the group is spliced into the enclosing stack.
pub fn group(children: Vec<Node>) -> Node {
    Node::Group(flatten(children))
}
