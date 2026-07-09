//! Stack layout solver.
//!
//! SignalKit relies on `UIStackView` + Auto Layout. The port replaces that with
//! a single-pass flexbox-lite solver: children are measured for their natural
//! size, laid out sequentially along the stack's main axis (with spacing), and
//! any leftover main-axis space is absorbed by flexible [`Spacer`]s. Cross-axis
//! placement follows the stack's [`Align`]. Layout writes each widget's
//! `frame`; nothing is drawn here.

use crate::geometry::{Rect, Size};
use crate::widget::stack::{join, split, StackState};
use crate::widget::{Align, AnyWidget, Axis};

/// Lays out `root` (and all descendants) to fill `bounds`.
pub fn layout(root: &AnyWidget, bounds: Rect) {
    place(root, bounds);
}

fn place(w: &AnyWidget, rect: Rect) {
    w.set_frame(rect);
    if let AnyWidget::Stack(s) = w {
        let state = s.0.borrow();
        place_stack(&state, rect);
    }
}

fn place_stack(s: &StackState, rect: Rect) {
    let inner = rect.inset(s.padding);
    let axis = s.axis;
    let n = s.children.len();
    if n == 0 {
        return;
    }

    // Natural main extents + total, and count of flexible children.
    let mut naturals: Vec<Size> = Vec::with_capacity(n);
    let mut total_main = 0;
    let mut flex_count = 0;
    for (i, child) in s.children.iter().enumerate() {
        let m = child.measure(inner.size());
        let (cm, _) = split(axis, m);
        naturals.push(m);
        total_main += cm;
        if child.is_flexible() {
            flex_count += 1;
        }
        if i + 1 < n {
            total_main += s.spacing;
        }
    }

    let (main_origin, main_size, cross_origin, cross_size) = axis_bounds(axis, inner);
    let leftover = (main_size - total_main).max(0);

    // Distribute leftover among flexible children (integer split, remainder to
    // the first few).
    let (per_flex, mut rem) = if flex_count > 0 {
        (leftover / flex_count, leftover % flex_count)
    } else {
        (0, 0)
    };

    let mut cursor = main_origin;
    for (child, natural) in s.children.iter().zip(&naturals) {
        let (nat_main, nat_cross) = split(axis, *natural);
        let mut child_main = nat_main;
        if child.is_flexible() {
            child_main += per_flex;
            if rem > 0 {
                child_main += 1;
                rem -= 1;
            }
        }

        let (child_cross_pos, child_cross_ext) =
            cross_placement(s.align, cross_origin, cross_size, nat_cross);

        let child_rect = rect_from(axis, cursor, child_main, child_cross_pos, child_cross_ext);
        place(child, child_rect);
        cursor += child_main + s.spacing;
    }
}

/// (main_origin, main_size, cross_origin, cross_size) of a rectangle for the
/// given axis.
fn axis_bounds(axis: Axis, r: Rect) -> (i32, i32, i32, i32) {
    match axis {
        Axis::Vertical => (r.y, r.h, r.x, r.w),
        Axis::Horizontal => (r.x, r.w, r.y, r.h),
    }
}

fn rect_from(axis: Axis, main_pos: i32, main_ext: i32, cross_pos: i32, cross_ext: i32) -> Rect {
    let size = join(axis, main_ext, cross_ext);
    match axis {
        Axis::Vertical => Rect::new(cross_pos, main_pos, size.w, size.h),
        Axis::Horizontal => Rect::new(main_pos, cross_pos, size.w, size.h),
    }
}

/// Cross-axis position and extent for a child given the stack alignment.
fn cross_placement(align: Align, origin: i32, avail: i32, natural: i32) -> (i32, i32) {
    match align {
        Align::Fill => (origin, avail),
        Align::Leading => (origin, natural),
        Align::Trailing => (origin + (avail - natural).max(0), natural),
        Align::Center => (origin + (avail - natural).max(0) / 2, natural),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::{font, Label, Spacer, Stack};

    fn label(text: &str) -> AnyWidget {
        AnyWidget::Label(Label::new(text))
    }

    #[test]
    fn vertical_stack_stacks_children_top_down_with_spacing() {
        let stack = Stack::vertical(10, Align::Fill);
        stack.push(label("aa")); // 2 cols -> 16 wide, 16 tall
        stack.push(label("bbbb")); // 4 cols -> 32 wide, 16 tall
        let root = AnyWidget::Stack(stack);
        layout(&root, Rect::new(0, 0, 100, 200));

        let children = root.children();
        assert_eq!(children[0].frame(), Rect::new(0, 0, 100, font::CELL_H));
        // Second child starts after first + spacing.
        assert_eq!(
            children[1].frame(),
            Rect::new(0, font::CELL_H + 10, 100, font::CELL_H)
        );
    }

    #[test]
    fn spacer_pushes_trailing_child_to_the_end() {
        let stack = Stack::vertical(0, Align::Fill);
        stack.push(label("top"));
        stack.push(AnyWidget::Spacer(Spacer::new()));
        stack.push(label("bot"));
        let root = AnyWidget::Stack(stack);
        layout(&root, Rect::new(0, 0, 80, 300));

        let children = root.children();
        assert_eq!(children[0].frame().y, 0);
        // Last child sits flush at the bottom (top of it = 300 - CELL_H).
        assert_eq!(children[2].frame().y, 300 - font::CELL_H);
    }

    #[test]
    fn horizontal_center_alignment_centers_cross_axis() {
        let stack = Stack::horizontal(0, Align::Center);
        stack.push(label("x")); // 8x16
        let root = AnyWidget::Stack(stack);
        layout(&root, Rect::new(0, 0, 200, 100));
        // Cross axis is vertical: child centered in 100 tall -> y = (100-16)/2.
        assert_eq!(root.children()[0].frame().y, (100 - font::CELL_H) / 2);
    }

    #[test]
    fn padding_insets_children() {
        let stack = Stack::vertical(0, Align::Fill).padding(5);
        stack.push(label("hi"));
        let root = AnyWidget::Stack(stack);
        layout(&root, Rect::new(0, 0, 100, 100));
        let f = root.children()[0].frame();
        assert_eq!((f.x, f.y, f.w), (5, 5, 90));
    }
}
