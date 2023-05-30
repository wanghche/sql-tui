use crate::{app::DialogResult, event::Key};
use std::cmp::min;
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

pub struct MultiSelect {
    title: String,
    state: ListState,
    options: Vec<String>,
    selected: Vec<String>,
}

impl MultiSelect {
    pub fn new(title: String, options: Vec<String>, selected: &Vec<String>) -> Self {
        MultiSelect {
            title,
            state: ListState::default(),
            options,
            selected: selected.to_vec(),
        }
    }
    pub fn draw<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        let bounds = f.size();
        let width = min(bounds.width - 2, 40);
        let height = 20;
        let left = (bounds.width - width) / 2;
        let top = (bounds.height - height) / 2;
        let rect = Rect::new(left, top, width, height);

        f.render_widget(Clear, rect);
        let items: Vec<ListItem> = self
            .options
            .iter()
            .map(|option| {
                let check = if self
                    .selected
                    .iter()
                    .find(|select| *select == option)
                    .is_some()
                {
                    "\u{2705}"
                } else {
                    "\u{274E}"
                };
                ListItem::new(Span::raw("{} {}", check, option));
            })
            .collect();
        f.render_stateful_widget(
            List::new(items)
                .block(
                    Block::default()
                        .title(self.title.as_str())
                        .borders(Borders::ALL),
                )
                .highlight_style(Style::default().add_modifier(Modifier::BOLD)),
            rect,
            &mut self.state,
        );
    }
    pub fn handle_event(&mut self, key: &Key) -> DialogResult<&Vec<String>> {
        match key {
            Key::Up => self.handle_up(),
            Key::Down => self.handle_down(),
            Key::Space => {
                let index = self.state.selected();
                if let Some(index) = index {
                    let option = &self.options[index];
                    let index = self
                        .selected
                        .iter()
                        .position(|selected| *selected == *option);
                    if let Some(i) = index {
                        self.selected.remove(i);
                    } else {
                        self.selected.push(option.to_string());
                    }
                }
            }
            Key::Esc => return DialogResult::Cancel,
            Key::Enter => return DialogResult::Confirm(&self.selected),
            _ => (),
        }
        DialogResult::Done
    }
    pub fn handle_up(&mut self) {
        if self.options.len() > 0 {
            let index = self.state.selected().unwrap_or_default();
            let new_index = if index >= 1 { index - 1 } else { 0 };
            self.state.select(Some(new_index));
        }
    }
    pub fn handle_down(&mut self) {
        if self.options.len() > 0 {
            let index = self.state.selected().unwrap_or_default();
            let new_index = if index < self.options.len() {
                index + 1
            } else {
                0
            };
            self.state.select(Some(new_index));
        }
    }
}
