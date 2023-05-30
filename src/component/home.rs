use crate::{
    app::{ComponentResult, Focus},
    event::{config::*, Key},
};
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

pub struct HomeComponent {}

impl HomeComponent {
    pub fn new() -> Self {
        HomeComponent {}
    }
    pub fn draw<B>(&self, f: &mut Frame<B>, r: Rect, is_focus: bool)
    where
        B: Backend,
    {
        f.render_widget(
            Block::default()
                .title("Home")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(if is_focus {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                }),
            r,
        );

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 7),
                Constraint::Ratio(5, 7),
                Constraint::Ratio(1, 7),
            ])
            .split(r.inner(&Margin {
                vertical: 1,
                horizontal: 1,
            }));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
            .split(chunks[1]);

        f.render_widget(
            Paragraph::new(vec![
                Spans::from(Span::styled("sql-tui", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                Spans::from("v0.0.1"),
                Spans::from("\n"),
                Spans::from("Sql-tui is a tui db client tool now support MySql/Postgres database.You can create|delete|edit database|tables|query|view|user like a gui db client on Windows|Mac|Linux."),
                Spans::from("If you didn't have a connection,create it first on left panel.Any command you can use please see the command bar which on top of screen.We suggest to use it in fullscreen terminal window."),
                Spans::from("\n"),
                Spans::from("repository: https://github.com/wanghche/sql-tui"),
                Spans::from("\n"),
                Spans::from("email: wangch@gmail.com"),
            ])
            .wrap(Wrap{ trim: false })
            .alignment(Alignment::Center),
            chunks[1],
        );
    }
    pub fn handle_event(&self, key: &Key) -> ComponentResult {
        match *key {
            LEFT_KEY => ComponentResult::Focus(Focus::LeftPanel),
            _ => ComponentResult::Done,
        }
    }
}
