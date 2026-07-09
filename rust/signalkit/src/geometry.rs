//! Integer pixel geometry. E-ink framebuffers are addressed in whole pixels, so
//! unlike UIKit's `CGFloat` everything here is `i32`.

/// A width/height pair in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    pub w: i32,
    pub h: i32,
}

impl Size {
    pub const fn new(w: i32, h: i32) -> Self {
        Size { w, h }
    }
    pub const ZERO: Size = Size { w: 0, h: 0 };
}

/// A point in pixels, origin top-left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }
}

/// An axis-aligned rectangle in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect { x, y, w, h }
    }

    pub const fn from_origin_size(origin: Point, size: Size) -> Self {
        Rect {
            x: origin.x,
            y: origin.y,
            w: size.w,
            h: size.h,
        }
    }

    pub const ZERO: Rect = Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };

    pub fn size(&self) -> Size {
        Size::new(self.w, self.h)
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    pub fn area(&self) -> i64 {
        (self.w.max(0) as i64) * (self.h.max(0) as i64)
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    /// True if `p` lies within the rectangle (left/top inclusive, right/bottom
    /// exclusive).
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }

    /// The smallest rectangle containing both `self` and `other`. Empty
    /// rectangles are ignored so `union` acts as a damage accumulator.
    pub fn union(&self, other: Rect) -> Rect {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return *self;
        }
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Rect::new(x, y, right - x, bottom - y)
    }

    /// The overlapping region of the two rectangles ([`Rect::ZERO`] if they
    /// don't intersect).
    pub fn intersection(&self, other: Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        if right <= x || bottom <= y {
            Rect::ZERO
        } else {
            Rect::new(x, y, right - x, bottom - y)
        }
    }

    /// True if `other` lies entirely within `self`. Empty rectangles are
    /// contained by anything.
    pub fn contains_rect(&self, other: Rect) -> bool {
        other.is_empty()
            || (other.x >= self.x
                && other.y >= self.y
                && other.right() <= self.right()
                && other.bottom() <= self.bottom())
    }

    /// True if the two rectangles overlap at all.
    pub fn intersects(&self, other: Rect) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    /// Insets the rectangle on all sides by `pad` (clamped at zero size).
    pub fn inset(&self, pad: i32) -> Rect {
        Rect::new(
            self.x + pad,
            self.y + pad,
            (self.w - 2 * pad).max(0),
            (self.h - 2 * pad).max(0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_is_half_open() {
        let r = Rect::new(10, 10, 20, 20);
        assert!(r.contains(Point::new(10, 10)));
        assert!(r.contains(Point::new(29, 29)));
        assert!(!r.contains(Point::new(30, 10)));
        assert!(!r.contains(Point::new(9, 10)));
    }

    #[test]
    fn union_ignores_empty() {
        let a = Rect::new(5, 5, 10, 10);
        assert_eq!(a.union(Rect::ZERO), a);
        assert_eq!(Rect::ZERO.union(a), a);
    }

    #[test]
    fn union_bounds_both() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(20, 5, 10, 30);
        assert_eq!(a.union(b), Rect::new(0, 0, 30, 35));
    }

    #[test]
    fn intersection_clips_to_overlap() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(5, 5, 10, 10);
        assert_eq!(a.intersection(b), Rect::new(5, 5, 5, 5));
        // Disjoint rectangles intersect to zero.
        assert_eq!(a.intersection(Rect::new(20, 20, 5, 5)), Rect::ZERO);
    }

    #[test]
    fn contains_rect_requires_full_coverage() {
        let outer = Rect::new(0, 0, 10, 10);
        assert!(outer.contains_rect(Rect::new(2, 2, 5, 5)));
        assert!(outer.contains_rect(outer));
        assert!(!outer.contains_rect(Rect::new(5, 5, 10, 10)));
        // Empty rects are contained by anything.
        assert!(outer.contains_rect(Rect::ZERO));
    }
}
