use crate::{
    app::DialogResult,
    component::Command,
    event::{config::*, Key, KeyCode},
    widget::form_item::FormItemResult,
};
use std::cmp::min;
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState},
    Frame,
};

pub struct Select {
    title: String,
    state: ListState,
    options: Vec<String>,
    match_str: String,
}

impl Select {
    pub fn new(title: String, options: Vec<String>, selected: Option<&str>) -> Self {
        let mut state = ListState::default();
        if let Some(selected) = selected {
            let index = options.iter().position(|option| option == selected);
            state.select(index);
        }

        Select {
            title,
            state,
            options,
            match_str: String::new(),
        }
    }
    pub fn draw<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        let bounds = f.size();
        let width = min(bounds.width - 2, 40);
        let height = bounds.height / 4;
        let left = (bounds.width - width) / 2;
        let top = (bounds.height - height) / 2;
        let rect = Rect::new(left, top, width, height);

        f.render_widget(Clear, rect);
        let items: Vec<ListItem> = self
            .options
            .iter()
            .map(|option| ListItem::new(option.as_str()))
            .collect();
        f.render_stateful_widget(
            List::new(items)
                .block(
                    Block::default()
                        .title(self.title.as_str())
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                )
                .highlight_style(Style::default().fg(Color::Green)),
            rect,
            &mut self.state,
        );
    }
    pub fn handle_event(&mut self, key: &Key) -> DialogResult<&str> {
        match *key {
            UP_KEY => self.handle_up(),
            DOWN_KEY => self.handle_down(),
            CONFIRM_KEY => {
                if let Some(i) = self.state.selected() {
                    self.match_str = String::new();
                    return DialogResult::Confirm(self.options[i].as_str());
                }
            }
            CANCEL_KEY => {
                return DialogResult::Cancel;
            }
            Key {
                code: KeyCode::Char(c),
                ..
            } => {
                self.match_str.push(c);
                let index = self
                    .options
                    .iter()
                    .position(|opt| opt.starts_with(self.match_str.as_str()));
                if let Some(i) = index {
                    self.state.select(Some(i));
                } else {
                    self.state.select(None);
                    self.match_str = String::new();
                }
            }
            _ => (),
        }
        DialogResult::Done
    }
    pub fn get_commands(&self) -> Vec<Command> {
        vec![
            Command {
                name: "Up",
                key: UP_KEY,
            },
            Command {
                name: "Down",
                key: DOWN_KEY,
            },
            Command {
                name: "Cancel",
                key: CANCEL_KEY,
            },
            Command {
                name: "Ok",
                key: CONFIRM_KEY,
            },
        ]
    }
    pub fn handle_up(&mut self) {
        if !self.options.is_empty() {
            let index = self.state.selected().unwrap_or_default();
            let new_index = if index >= 1 { index - 1 } else { 0 };
            self.state.select(Some(new_index));
        }
    }
    pub fn handle_down(&mut self) {
        if !self.options.is_empty() {
            let index = if let Some(i) = self.state.selected() {
                min(i + 1, self.options.len() - 1)
            } else {
                0
            };
            self.state.select(Some(index));
        }
    }
}
pub fn draw_select<B>(f: &mut Frame<B>, title: &str, options: &[String], state: &mut ListState)
where
    B: Backend,
{
    let bounds = f.size();
    let width = min(bounds.width - 2, 40);
    let height = bounds.height / 4;
    let left = (bounds.width - width) / 2;
    let top = (bounds.height - height) / 2;
    let rect = Rect::new(left, top, width, height);

    f.render_widget(Clear, rect);
    let items: Vec<ListItem> = options
        .iter()
        .map(|option| ListItem::new(option.as_str()))
        .collect();
    f.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .highlight_style(Style::default().fg(Color::Green)),
        rect,
        state,
    );
}
pub fn handle_select_event(
    name: &str,
    options: &Vec<String>,
    selected: &mut Option<String>,
    nullable: &bool,
    is_pop: &mut bool,
    state: &mut ListState,
    match_str: &mut String,
    key: &Key,
) -> FormItemResult {
    if *is_pop {
        match *key {
            UP_KEY => {
                if !options.is_empty() {
                    let index = state.selected().unwrap_or_default();
                    let new_index = if index >= 1 { index - 1 } else { 0 };

                    state.select(Some(new_index));
                    *match_str = String::new();
                }
                FormItemResult::Handled
            }
            DOWN_KEY => {
                if !options.is_empty() {
                    let index = if let Some(i) = state.selected() {
                        min(i + 1, options.len() - 1)
                    } else {
                        0
                    };
                    state.select(Some(index));
                    *match_str = String::new();
                }
                FormItemResult::Handled
            }
            CONFIRM_KEY => {
                let index = state.selected();
                if let Some(i) = index {
                    *match_str = String::new();
                    if *selected != Some(options[i].clone()) {
                        *selected = Some(options[i].clone());
                        *is_pop = false;
                        return FormItemResult::Changed(name.to_string(), options[i].clone());
                    }
                }
                *is_pop = false;
                FormItemResult::Handled
            }
            CANCEL_KEY => {
                *is_pop = false;
                *match_str = String::new();
                FormItemResult::Handled
            }
            Key {
                code: KeyCode::Char(c),
                ..
            } => {
                match_str.push(c);
                let index = options
                    .iter()
                    .position(|opt| opt.starts_with(match_str.as_str()));
                if let Some(i) = index {
                    state.select(Some(i));
                } else {
                    state.select(None);
                    *match_str = String::new();
                }
                FormItemResult::Handled
            }
            _ => FormItemResult::UnHandled,
        }
    } else {
        match *key {
            CONFIRM_KEY => {
                *is_pop = true;
                FormItemResult::Handled
            }
            CLEAR_KEY => {
                if *nullable {
                    *selected = None;
                }
                FormItemResult::Handled
            }
            _ => FormItemResult::UnHandled,
        }
    }
}
