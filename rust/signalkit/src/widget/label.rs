//! Text label widget.

use std::cell::RefCell;
use std::rc::Rc;

use crate::geometry::Size;
use crate::render::{Color, DrawCmd};
use crate::widget::{font, Common};

pub(crate) struct LabelState {
    pub common: Common,
    pub text: String,
    pub size: u8,
    pub fg: Color,
    pub bg: Color,
}

impl LabelState {
    pub(crate) fn measure(&self) -> Size {
        font::text_size(&self.text, self.size)
    }

    pub(crate) fn paint(&self, out: &mut Vec<DrawCmd>) {
        let f = self.common.frame;
        out.push(DrawCmd::Text {
            x: f.x,
            y: f.y,
            text: self.text.clone(),
            size: self.size,
            fg: self.fg,
            bg: self.bg,
            inverse: false,
        });
    }
}

/// A single-line text label. Cheap to clone (shares state).
#[derive(Clone)]
pub struct Label(pub(crate) Rc<RefCell<LabelState>>);

impl Label {
    /// Creates a black-on-white label at size multiplier 1.
    pub fn new(text: impl Into<String>) -> Self {
        Label(Rc::new(RefCell::new(LabelState {
            common: Common::new(),
            text: text.into(),
            size: 1,
            fg: Color::BLACK,
            bg: Color::WHITE,
        })))
    }

    /// Sets the font size multiplier (builder style).
    pub fn size(self, size: u8) -> Self {
        self.0.borrow_mut().size = size.max(1);
        self
    }

    /// Sets the foreground/background colors (builder style).
    pub fn colors(self, fg: Color, bg: Color) -> Self {
        {
            let mut s = self.0.borrow_mut();
            s.fg = fg;
            s.bg = bg;
        }
        self
    }

    /// Replaces the text and marks the label dirty. This is the target of most
    /// signal bindings.
    pub fn set_text(&self, text: impl Into<String>) {
        let mut s = self.0.borrow_mut();
        let text = text.into();
        if s.text != text {
            s.text = text;
            s.common.dirty = true;
        }
    }

    /// Reads the current text.
    pub fn text(&self) -> String {
        self.0.borrow().text.clone()
    }
}
