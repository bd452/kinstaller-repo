//! Host-side recording renderer used by tests.

use crate::geometry::{Rect, Size};
use crate::render::{DrawCmd, Renderer, RefreshMode};

/// A [`Renderer`] that records everything submitted to it instead of drawing.
/// Tests assert on [`MockRenderer::commands`] and [`MockRenderer::refreshes`].
pub struct MockRenderer {
    size: Size,
    pub commands: Vec<DrawCmd>,
    pub refreshes: Vec<(Rect, RefreshMode)>,
}

impl MockRenderer {
    pub fn new(size: Size) -> Self {
        MockRenderer {
            size,
            commands: Vec::new(),
            refreshes: Vec::new(),
        }
    }

    /// Clears the recorded log (call between frames in a test).
    pub fn clear_log(&mut self) {
        self.commands.clear();
        self.refreshes.clear();
    }

    /// Convenience: all text strings drawn so far, in order.
    pub fn texts(&self) -> Vec<String> {
        self.commands
            .iter()
            .filter_map(|c| match c {
                DrawCmd::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }
}

impl Renderer for MockRenderer {
    fn screen_size(&mut self) -> Size {
        self.size
    }

    fn submit(&mut self, cmds: &[DrawCmd]) -> std::io::Result<()> {
        self.commands.extend_from_slice(cmds);
        Ok(())
    }

    fn refresh(&mut self, region: Rect, mode: RefreshMode) -> std::io::Result<()> {
        self.refreshes.push((region, mode));
        Ok(())
    }
}
