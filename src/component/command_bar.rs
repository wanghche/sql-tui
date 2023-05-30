use crate::event::{config::*, Key};
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Tabs},
    Frame,
};

#[derive(Clone)]
pub struct Command {
    pub name: &'static str,
    pub key: Key,
}

pub struct CommandBarComponent {
    commands: Vec<Command>,
}

impl CommandBarComponent {
    pub fn new() -> CommandBarComponent {
        CommandBarComponent { commands: vec![] }
    }
    pub fn draw<B>(&self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let tabs = Tabs::new(
            self.commands
                .iter()
                .map(|c| {
                    Spans::from(Span::styled(
                        format!("{} [{}]", c.name, c.key),
                        Style::default().add_modifier(Modifier::BOLD),
                    ))
                })
                .collect(),
        )
        .block(Block::default().borders(Borders::BOTTOM));

        f.render_widget(tabs, r);
    }
    pub fn set_commands(&mut self, cmds: &mut Vec<Command>) {
        cmds.push(Command {
            name: "Quit App",
            key: QUIT_APP_KEY,
        });
        self.commands = cmds.to_vec();
    }
}
