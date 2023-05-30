use crate::{
    app::DialogResult,
    component::Command,
    event::{config::*, Key},
    widget::{DialogState, FormItem, FormItemResult},
};
use anyhow::{Error, Result};
use std::collections::HashMap;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    widgets::{Block, BorderType, Borders},
    Frame,
};

const MARGIN_VERTICAL: u16 = 1;
const MARGIN_HORIZONTAL: u16 = 1;

#[derive(Default, Clone)]
pub struct Form<'a> {
    title: String,
    focus: usize,
    offset: usize,
    items: Vec<FormItem<'a>>,
}

impl<'a> Form<'a> {
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }
    pub fn set_items(&mut self, items: Vec<FormItem<'a>>) {
        self.items = items;
    }
    pub fn get_item(&self, name: &str) -> Option<&FormItem<'a>> {
        self.items.iter().find(|field| field.name() == name)
    }
    pub fn get_item_mut(&mut self, name: &str) -> Option<&mut FormItem<'a>> {
        self.items.iter_mut().find(|field| field.name() == name)
    }
    pub fn get_value(&self, name: &str) -> Option<String> {
        let item = self
            .items
            .iter()
            .find(|field| field.name() == name)
            .expect("cann't find this item");
        item.get_value()
    }
    pub fn set_item(&mut self, name: &str, item: FormItem<'a>) {
        let index = self.items.iter().position(|i| i.name() == name);
        if let Some(index) = index {
            self.items.splice(index..index + 1, [item]);
        }
    }
    pub fn set_value(&mut self, name: &str, value: &str) {
        let index = self.items.iter().position(|i| i.name() == name);
        if let Some(index) = index {
            self.items[index].set_value(value);
        }
    }
    pub fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        f.render_widget(
            Block::default()
                .title(self.title.as_str())
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
            r,
        );
        let inner = r.inner(&Margin {
            horizontal: MARGIN_HORIZONTAL,
            vertical: MARGIN_VERTICAL,
        });
        let (start, end) = self.get_items_bounds(self.focus, self.offset, inner.height as usize);
        self.offset = start;
        let items_count = inner.height / 3;
        let items_height = items_count * 3;
        let mut constraints = vec![Constraint::Length(3); items_count as usize];
        if inner.height > items_height {
            constraints.push(Constraint::Min(1));
        }
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        self.items
            .iter_mut()
            .skip(self.offset)
            .take(end - start)
            .enumerate()
            .for_each(|(i, item)| {
                item.draw(f, chunks[i], self.focus - self.offset == i);
            });
        self.items
            .iter_mut()
            .skip(self.offset)
            .take(end - start)
            .enumerate()
            .for_each(|(_i, item)| {
                item.draw_dialog(f);
            });
    }
    pub fn get_commands(&self) -> Vec<Command> {
        let mut cmds = vec![
            Command {
                name: "Up",
                key: UP_KEY,
            },
            Command {
                name: "Down",
                key: DOWN_KEY,
            },
        ];
        if let Some(focus_item) = self.get_focus_item() {
            let item_cmds = focus_item.get_commands();
            match focus_item {
                FormItem::Select { is_pop, .. } => {
                    if *is_pop {
                        cmds = item_cmds;
                    } else {
                        cmds.extend(item_cmds);
                    }
                }
                FormItem::MultiSelect { is_pop, .. } => {
                    if *is_pop {
                        cmds = item_cmds;
                    } else {
                        cmds.extend(item_cmds);
                    }
                }
                FormItem::TextArea { is_pop, .. } => {
                    if *is_pop {
                        cmds = item_cmds;
                    } else {
                        cmds.extend(item_cmds);
                    }
                }
                FormItem::List { dlg_state, .. } => match dlg_state {
                    DialogState::None => {
                        cmds.extend(item_cmds);
                    }
                    _ => cmds = item_cmds,
                },
                FormItem::TableList { dlg_state, .. } => match dlg_state {
                    DialogState::None => {
                        cmds.extend(item_cmds);
                    }
                    _ => cmds = item_cmds,
                },
                _ => cmds.extend(item_cmds),
            }
        }
        cmds
    }
    pub fn get_focus_item_mut(&mut self) -> Option<&mut FormItem<'a>> {
        self.items.get_mut(self.focus)
    }
    pub fn get_focus_item(&self) -> Option<&FormItem<'a>> {
        self.items.get(self.focus)
    }
    pub fn height(&self) -> u16 {
        (self.items.len() * 3) as u16 + MARGIN_VERTICAL * 2
    }
    pub fn handle_event(
        &mut self,
        key: &Key,
    ) -> Result<DialogResult<HashMap<String, Option<String>>>> {
        match self.items[self.focus].handle_event(key)? {
            FormItemResult::Changed(name, changed) => {
                return Ok(DialogResult::Changed(name, changed));
            }
            FormItemResult::UnHandled => match *key {
                UP_KEY => {
                    if self.focus != 0 {
                        self.focus -= 1;
                    }
                }
                DOWN_KEY => {
                    if self.focus < self.items.len() - 1 {
                        self.focus += 1;
                    }
                }
                CANCEL_KEY => {
                    return Ok(DialogResult::Cancel);
                }
                SAVE_KEY => {
                    self.validate_input()?;
                    return Ok(DialogResult::Confirm(self.get_data()));
                }
                _ => (),
            },
            FormItemResult::Handled => (),
        }
        Ok(DialogResult::Done)
    }
    pub fn clear(&mut self) {
        self.items.iter_mut().for_each(|item| item.clear());
        self.focus = 0;
        self.offset = 0;
    }
    pub fn validate_input(&self) -> Result<()> {
        for item in self.items.iter() {
            match &item {
                FormItem::Input {
                    name,
                    nullable,
                    can_null,
                    is_null,
                    ..
                } => {
                    if !*nullable {
                        if *can_null && *is_null {
                            return Err(Error::msg(format!("Please input {}", name)));
                        }
                    }
                }
                FormItem::Select {
                    name,
                    selected,
                    nullable,
                    ..
                } => {
                    if !*nullable && selected.is_none() {
                        return Err(Error::msg(format!("Please select {}", name)));
                    }
                }
                FormItem::MultiSelect {
                    name,
                    selected,
                    nullable,
                    ..
                } => {
                    if !nullable && selected.is_empty() {
                        return Err(Error::msg(format!("Please select {}", name)));
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }
    pub fn get_data(&self) -> HashMap<String, Option<String>> {
        let mut map: HashMap<String, Option<String>> = HashMap::new();
        for item in self.items.iter() {
            match item {
                FormItem::Input {
                    name,
                    input,
                    can_null,
                    is_null,
                    ..
                } => {
                    map.insert(
                        name.to_string(),
                        if *can_null && *is_null {
                            None
                        } else {
                            Some(input.lines().join(""))
                        },
                    );
                }
                FormItem::TextArea {
                    name,
                    textarea,
                    can_null,
                    is_null,
                    ..
                } => {
                    map.insert(
                        name.to_string(),
                        if *can_null && *is_null {
                            None
                        } else {
                            Some(textarea.lines().join("\n"))
                        },
                    );
                }
                FormItem::Select { name, selected, .. } => {
                    if let Some(s) = selected {
                        map.insert(name.to_string(), Some(s.to_owned()));
                    }
                }
                FormItem::Check { name, checked, .. } => {
                    map.insert(name.to_string(), Some(checked.to_string()));
                }
                FormItem::MultiSelect { name, selected, .. } => {
                    if !selected.is_empty() {
                        map.insert(name.to_string(), Some(selected.join(",")));
                    }
                }
                FormItem::List { name, items, .. } => {
                    map.insert(name.to_string(), Some(items.join(",").to_string()));
                }
                FormItem::TableList { name, rows, .. } => {
                    map.insert(
                        name.to_string(),
                        Some(
                            rows.iter()
                                .map(|row| row.join(":"))
                                .collect::<Vec<String>>()
                                .join(",")
                                .to_string(),
                        ),
                    );
                }
            }
        }
        map
    }
    fn get_items_bounds(&self, focus: usize, offset: usize, max_height: usize) -> (usize, usize) {
        let offset = offset.min(self.items.len().saturating_sub(1));
        let mut start = offset;
        let mut end = offset;
        let mut height = 0;
        for item in self.items.iter().skip(offset) {
            if height + item.height() > max_height {
                break;
            }
            height += item.height();
            end += 1;
        }
        let focus = focus.min(self.items.len() - 1);
        while focus >= end {
            height = height.saturating_add(self.items[end].height());
            end += 1;
            while height > max_height {
                height = height.saturating_sub(self.items[start].height());
                start += 1;
            }
        }
        while focus < start {
            start -= 1;
            height = height.saturating_add(self.items[start].height());
            while height > max_height {
                end -= 1;
                height = height.saturating_sub(self.items[end].height());
            }
        }
        (start, end)
    }
}
