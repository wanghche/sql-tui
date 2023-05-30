use crate::{
    app::DialogResult,
    component::Command,
    event::{config::*, Key},
};
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear},
    Frame,
};
use tui_textarea::{Input, TextArea};

pub struct InputDialog<'a> {
    title: String,
    input: TextArea<'a>,
}

impl<'a> InputDialog<'a> {
    pub fn new(title: &'a str, content: Option<&str>) -> Self {
        let mut input = if let Some(s) = content {
            TextArea::from([s])
        } else {
            TextArea::default()
        };
        input.set_block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
                .border_type(BorderType::Rounded),
        );

        InputDialog {
            title: title.to_string(),
            input,
        }
    }
    pub fn draw<B>(&self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        let bounds = f.size();
        let width = std::cmp::min(bounds.width - 2, 60);
        let height = 3;
        let left = (bounds.width - width) / 2;
        let top = (bounds.height - height) / 2;
        let rect = Rect::new(left, top, width, height);
        f.render_widget(Clear, rect);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(self.title.as_str());

        f.render_widget(block, rect);
        f.render_widget(self.input.widget(), rect);
    }
    pub fn handle_event(&mut self, key: &Key) -> DialogResult<String> {
        match *key {
            CANCEL_KEY => {
                return DialogResult::Cancel;
            }
            SAVE_KEY => {
                return DialogResult::Confirm(self.input.lines().join("\n"));
            }
            _ => {
                let input: Input = key.to_owned().into();
                self.input.input(input);
            }
        }
        DialogResult::Done
    }
    pub fn get_commands(&self) -> Vec<Command> {
        vec![
            Command {
                name: "Cancel",
                key: CANCEL_KEY,
            },
            Command {
                name: "Save",
                key: SAVE_KEY,
            },
        ]
    }
}
