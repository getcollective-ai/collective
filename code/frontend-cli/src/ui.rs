use tui::{backend::Backend, Frame};

use crate::widget::Label;

pub struct Ui {
    input: Vec<String>,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            input: vec![String::new()],
        }
    }

    pub fn reset(&mut self) {
        self.input.clear();
        self.input.push(String::new());
    }

    pub fn current_line(&mut self) -> &mut String {
        self.input.last_mut().unwrap()
    }

    pub fn new_line(&mut self) {
        self.input.push(String::new());
    }

    pub fn run<B: Backend>(&self, f: &mut Frame<B>) {
        let size = f.size();

        let mut render_loc = size;

        for i in 0..self.input.len() {
            let label = Label::default().text(&self.input[i]);
            f.render_widget(label, render_loc);
            render_loc.y += 1;
        }
        f.set_cursor(
            render_loc.x + u16::try_from(self.input.last().unwrap().len()).unwrap(),
            render_loc.y - 1,
        );
    }
}
