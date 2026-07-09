//! Tappable button widget.

use std::cell::RefCell;
use std::rc::Rc;

use crate::geometry::Size;
use crate::render::{Color, DrawCmd};
use crate::widget::{font, Common};

/// Padding between a button's title and its border, in pixels.
const PAD_X: i32 = 12;
const PAD_Y: i32 = 8;

pub(crate) struct ButtonState {
    pub common: Common,
    pub title: String,
    pub size: u8,
    pub fg: Color,
    pub bg: Color,
    pub on_tap: Option<Rc<dyn Fn()>>,
}

impl ButtonState {
    pub(crate) fn measure(&self) -> Size {
        let t = font::text_size(&self.title, self.size);
        Size::new(t.w + 2 * PAD_X, t.h + 2 * PAD_Y)
    }

    pub(crate) fn paint(&self, out: &mut Vec<DrawCmd>) {
        let f = self.common.frame;
        // Border/background: a filled rect in fg gives an inverse chip look that
        // reads clearly on e-ink; the title is drawn inverted on top.
        out.push(DrawCmd::FillRect {
            rect: f,
            color: self.fg,
        });
        out.push(DrawCmd::Text {
            x: f.x + PAD_X,
            y: f.y + PAD_Y,
            text: self.title.clone(),
            size: self.size,
            fg: self.fg,
            bg: self.bg,
            inverse: true,
        });
    }
}

/// A button with a text title and a tap handler. Cheap to clone (shares state).
#[derive(Clone)]
pub struct Button(pub(crate) Rc<RefCell<ButtonState>>);

impl Button {
    /// Creates a button with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Button(Rc::new(RefCell::new(ButtonState {
            common: Common::new(),
            title: title.into(),
            size: 1,
            fg: Color::BLACK,
            bg: Color::WHITE,
            on_tap: None,
        })))
    }

    /// Sets the font size multiplier (builder style).
    pub fn size(self, size: u8) -> Self {
        self.0.borrow_mut().size = size.max(1);
        self
    }

    /// Registers the tap handler (builder style). Replaces any previous one.
    pub fn on_tap(self, handler: impl Fn() + 'static) -> Self {
        self.0.borrow_mut().on_tap = Some(Rc::new(handler));
        self
    }

    /// Sets the title and marks the button dirty.
    pub fn set_title(&self, title: impl Into<String>) {
        let mut s = self.0.borrow_mut();
        let title = title.into();
        if s.title != title {
            s.title = title;
            s.common.dirty = true;
        }
    }

    /// Runs the tap handler if set. The handler is cloned out of the borrow
    /// before calling so it may freely mutate this button. Returns whether a
    /// handler ran.
    pub(crate) fn fire_tap(&self) -> bool {
        let handler = self.0.borrow().on_tap.clone();
        if let Some(h) = handler {
            h();
            true
        } else {
            false
        }
    }
}
