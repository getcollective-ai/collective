use std::borrow::Cow;

use tui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};

#[derive(Default)]
pub struct Label<'a> {
    text: Cow<'a, str>,
}

impl<'a> Widget for Label<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_string(area.left(), area.top(), self.text, Style::default());
    }
}

impl<'a> Label<'a> {
    pub(crate) fn text(mut self, text: impl Into<Cow<'a, str>>) -> Label<'a> {
        self.text = text.into();
        self
    }
}
