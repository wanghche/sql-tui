use crate::{
    app::DialogResult,
    component::Command,
    event::{config::*, Key},
};
use tui::{
    backend::Backend,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub enum Kind {
    Warning,
    Error,
    Info,
    Confirm,
}

pub struct ConfirmDialog {
    kind: Kind,
    title: String,
    msg: String,
}

impl ConfirmDialog {
    pub fn new(kind: Kind, title: &str, msg: &str) -> Self {
        ConfirmDialog {
            kind,
            title: title.to_string(),
            msg: msg.to_string(),
        }
    }
    pub fn draw<B>(&self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        let main_color = match self.kind {
            Kind::Error => Color::Red,
            Kind::Warning => Color::Yellow,
            Kind::Info => Color::White,
            Kind::Confirm => Color::Gray,
        };

        let content = Text::styled(
            self.msg.as_str(),
            Style::default().fg(main_color).add_modifier(Modifier::BOLD),
        );

        let bounds = f.size();
        let width = std::cmp::min(bounds.width - 2, 45);
        let line_count = (content.width() as f64 / (width - 2) as f64).ceil() as u16;
        let height = line_count + 2;
        let left = (bounds.width - width) / 2;
        let top = (bounds.height - height) / 2;
        let rect = Rect::new(left, top, width, height);
        f.render_widget(Clear, rect);

        f.render_widget(
            Paragraph::new(content)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(main_color))
                        .border_type(BorderType::Rounded)
                        .title(self.title.as_str()),
                )
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center),
            rect,
        );
    }
    pub fn handle_event(&mut self, key: &Key) -> DialogResult<()> {
        match *key {
            CONFIRM_KEY => DialogResult::Confirm(()),
            CANCEL_KEY => DialogResult::Cancel,
            _ => DialogResult::Done,
        }
    }
    pub fn get_commands(&self) -> Vec<Command> {
        vec![
            Command {
                name: "Ok",
                key: CONFIRM_KEY,
            },
            Command {
                name: "Cancel",
                key: CANCEL_KEY,
            },
        ]
    }
}
