use crate::{
    component::Command,
    event::{config::*, Key, KeyCode, KeyModifier},
    widget::select::{draw_select, handle_select_event},
};
use anyhow::{Error, Result};
use std::cmp::min;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Row, Table,
        TableState,
    },
    Frame,
};
use tui_textarea::{CursorMove, Input, TextArea};

pub enum FormItemResult {
    Handled,
    UnHandled,
    Changed(String, String),
}

#[derive(Clone)]
pub enum ColumnInfo {
    Input {
        name: String,
        value: String,
        nullable: bool,
    },
    Select {
        name: String,
        selected: Option<String>,
        options: Vec<String>,
        state: ListState,
        is_pop: bool,
        nullable: bool,
        match_str: String,
    },
}
impl ColumnInfo {
    fn name(&self) -> &str {
        match self {
            ColumnInfo::Input { name, .. } => name.as_str(),
            ColumnInfo::Select { name, .. } => name.as_str(),
        }
    }
    fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect, is_focus: bool)
    where
        B: Backend,
    {
        let focus_style = Style::default().fg(Color::Green);
        let default_style = Style::default();

        match self {
            ColumnInfo::Select {
                name,
                selected,
                nullable,
                ..
            } => {
                let title = if *nullable {
                    name.clone()
                } else {
                    format!("{}(*)", name)
                };
                let widget = Paragraph::new(Span::styled(
                    selected.clone().unwrap_or_default(),
                    if is_focus { focus_style } else { default_style },
                ))
                .block(
                    Block::default()
                        .title(title.as_str())
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(if is_focus { focus_style } else { default_style }),
                );
                f.render_widget(widget, r);
            }
            ColumnInfo::Input {
                name,
                value,
                nullable,
                ..
            } => {
                let title = if *nullable {
                    name.clone()
                } else {
                    format!("{}(*)", name)
                };
                let widget = Paragraph::new(Span::styled(
                    value.clone(),
                    if is_focus { focus_style } else { default_style },
                ))
                .block(
                    Block::default()
                        .title(title.as_str())
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(if is_focus { focus_style } else { default_style }),
                );
                f.render_widget(widget, r);
            }
        }
    }
    fn draw_dialog<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        if let ColumnInfo::Select {
            name,
            options,
            is_pop,
            state,
            ..
        } = self
        {
            if *is_pop {
                draw_select(f, name, options, state);
            }
        }
    }
    fn handle_event(&mut self, key: &Key) -> Result<FormItemResult> {
        let result = match self {
            ColumnInfo::Select {
                name,
                options,
                selected,
                nullable,
                is_pop,
                state,
                match_str,
            } => handle_select_event(
                name, options, selected, nullable, is_pop, state, match_str, key,
            ),
            ColumnInfo::Input { value, .. } => match *key {
                CLEAR_KEY => {
                    value.pop();
                    FormItemResult::Handled
                }
                Key {
                    code: KeyCode::Char(c),
                    modifier: KeyModifier::None | KeyModifier::Shift,
                } => {
                    value.push(c);
                    FormItemResult::Handled
                }
                _ => FormItemResult::UnHandled,
            },
        };
        Ok(result)
    }
}

#[derive(Clone)]
pub enum DialogState {
    List,
    Input,
    None,
}

type ShowFn = fn(&[String]) -> String;

#[derive(Clone)]
pub enum FormItem<'a> {
    Select {
        name: String,
        options: Vec<String>,
        selected: Option<String>,
        state: ListState,
        match_str: String,
        is_pop: bool,
        nullable: bool,
        readonly: bool,
    },
    MultiSelect {
        name: String,
        options: Vec<String>,
        selected: Vec<String>,
        nullable: bool,
        state: ListState,
        is_pop: bool,
        list_selected: Vec<String>,
        readonly: bool,
    },
    Input {
        name: String,
        input: TextArea<'a>,
        nullable: bool,
        is_null: bool,
        can_null: bool,
        readonly: bool,
    },
    TextArea {
        name: String,
        textarea: TextArea<'a>,
        nullable: bool,
        is_pop: bool,
        is_null: bool,
        can_null: bool,
        readonly: bool,
    },
    List {
        name: String,
        items: Vec<String>,
        state: ListState,
        textarea: TextArea<'a>,
        dlg_state: DialogState,
        edit_index: Option<usize>,
        nullable: bool,
        readonly: bool,
    },
    TableList {
        name: String,
        rows: Vec<Vec<String>>,
        columns: Vec<ColumnInfo>,
        state: TableState,
        dlg_state: DialogState,
        edit_index: Option<usize>,
        focus: usize,
        offset: usize,
        nullable: bool,
        readonly: bool,
        show_fn: ShowFn,
    },
    Check {
        name: String,
        checked: bool,
        readonly: bool,
    },
}

impl<'a> FormItem<'a> {
    pub fn new_input(
        name: String,
        content: Option<&str>,
        nullable: bool,
        can_null: bool,
        readonly: bool,
    ) -> FormItem<'a> {
        let input = if let Some(content) = content {
            TextArea::from(content.lines())
        } else {
            if can_null {
                TextArea::from(["(NULL)"])
            } else {
                TextArea::default()
            }
        };

        FormItem::Input {
            name,
            input,
            nullable,
            is_null: can_null && content.is_none(),
            can_null,
            readonly,
        }
    }
    pub fn new_textarea(
        name: String,
        content: Option<&str>,
        nullable: bool,
        can_null: bool,
        readonly: bool,
    ) -> FormItem<'a> {
        let mut textarea = if let Some(content) = content {
            TextArea::from(content.lines())
        } else {
            TextArea::default()
        };
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );
        FormItem::TextArea {
            name,
            textarea,
            nullable,
            is_pop: false,
            is_null: can_null && content.is_none(),
            can_null,
            readonly,
        }
    }
    pub fn new_select(
        name: String,
        options: Vec<String>,
        selected: Option<String>,
        nullable: bool,
        readonly: bool,
    ) -> FormItem<'a> {
        let index = if let Some(s) = selected.as_ref() {
            options.iter().position(|o| o == s)
        } else {
            None
        };
        let mut state = ListState::default();
        state.select(index);
        FormItem::Select {
            name,
            options,
            selected,
            nullable,
            state,
            is_pop: false,
            match_str: String::new(),
            readonly,
        }
    }
    pub fn new_multi_select(
        name: String,
        options: Vec<String>,
        selected: Vec<String>,
        nullable: bool,
        readonly: bool,
    ) -> FormItem<'a> {
        FormItem::MultiSelect {
            name,
            options,
            selected,
            nullable,
            is_pop: false,
            state: ListState::default(),
            list_selected: Vec::new(),
            readonly,
        }
    }
    pub fn new_check(name: String, checked: bool, readonly: bool) -> FormItem<'a> {
        FormItem::Check {
            name,
            checked,
            readonly,
        }
    }
    pub fn new_list(
        name: String,
        items: Vec<String>,
        nullable: bool,
        readonly: bool,
    ) -> FormItem<'a> {
        FormItem::List {
            name,
            items,
            nullable,
            state: ListState::default(),
            textarea: TextArea::default(),
            dlg_state: DialogState::None,
            edit_index: None,
            readonly,
        }
    }
    pub fn new_table_list(
        name: String,
        rows: Vec<Vec<String>>,
        columns: Vec<ColumnInfo>,
        nullable: bool,
        show_fn: ShowFn,
        readonly: bool,
    ) -> FormItem<'a> {
        FormItem::TableList {
            name,
            rows,
            nullable,
            columns,
            state: TableState::default(),
            dlg_state: DialogState::None,
            edit_index: None,
            focus: 0,
            offset: 0,
            show_fn,
            readonly,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            FormItem::Select { name, .. } => name.as_str(),
            FormItem::MultiSelect { name, .. } => name.as_str(),
            FormItem::Input { name, .. } => name.as_str(),
            FormItem::TextArea { name, .. } => name.as_str(),
            FormItem::Check { name, .. } => name.as_str(),
            FormItem::List { name, .. } => name.as_str(),
            FormItem::TableList { name, .. } => name.as_str(),
        }
    }
    pub fn height(&self) -> usize {
        3
    }
    pub fn get_value(&self) -> Option<String> {
        match self {
            FormItem::Select { selected, .. } => selected.clone(),
            FormItem::MultiSelect { selected, .. } => Some(selected.join(",")),
            FormItem::Input {
                input,
                is_null,
                can_null,
                ..
            } => {
                if !input.is_empty() {
                    Some(input.lines().join(""))
                } else if *can_null && *is_null {
                    None
                } else {
                    Some(String::new())
                }
            }
            FormItem::TextArea {
                textarea,
                can_null,
                is_null,
                ..
            } => {
                if !textarea.is_empty() {
                    Some(textarea.lines().join("\n"))
                } else if *can_null && *is_null {
                    None
                } else {
                    Some(String::new())
                }
            }
            FormItem::Check { checked, .. } => Some(if *checked {
                "true".to_string()
            } else {
                "false".to_string()
            }),
            FormItem::List { items, .. } => Some(items.join(";")),
            _ => None,
        }
    }
    pub fn set_value(&mut self, value: &str) {
        match self {
            FormItem::Input { input, is_null, .. } => {
                while input.delete_newline() {
                    input.move_cursor(CursorMove::Down);
                }
                input.insert_str(value);
                *is_null = false;
            }
            FormItem::Select { selected, .. } => *selected = Some(value.to_string()),
            FormItem::Check { checked, .. } => {
                *checked = value == "true";
            }
            _ => (),
        }
    }
    pub fn clear(&mut self) {
        match self {
            FormItem::Select {
                selected,
                state,
                is_pop,
                ..
            } => {
                *selected = None;
                state.select(None);
                *is_pop = false;
            }
            FormItem::MultiSelect {
                selected,
                state,
                is_pop,
                list_selected,
                ..
            } => {
                *selected = Vec::new();
                state.select(None);
                *is_pop = false;
                *list_selected = Vec::new()
            }
            FormItem::Input { input, .. } => {
                while input.delete_newline() {
                    input.move_cursor(CursorMove::Down);
                }
            }
            FormItem::TextArea { textarea, .. } => {
                while textarea.delete_newline() {
                    textarea.move_cursor(CursorMove::Down);
                }
            }
            FormItem::List {
                items,
                state,
                textarea,
                ..
            } => {
                *items = Vec::new();
                state.select(None);
                while textarea.delete_newline() {
                    textarea.move_cursor(CursorMove::Down);
                }
            }
            FormItem::TableList {
                rows,
                state,
                dlg_state,
                focus,
                offset,
                ..
            } => {
                *rows = Vec::new();
                state.select(None);
                *dlg_state = DialogState::None;
                *focus = 0;
                *offset = 0;
            }
            FormItem::Check { checked, .. } => *checked = false,
        }
    }

    pub fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect, is_focus: bool)
    where
        B: Backend,
    {
        let focus_style = Style::default().fg(Color::Green);
        let readonly_style = Style::default().fg(Color::DarkGray);
        let default_style = Style::default();

        match self {
            FormItem::Select {
                name,
                selected,
                nullable,
                readonly,
                ..
            } => {
                let title = if *nullable {
                    name.clone()
                } else {
                    format!("{}(*)", name)
                };
                let widget = Paragraph::new(Span::styled(
                    selected.clone().unwrap_or_default(),
                    if *readonly {
                        readonly_style
                    } else if is_focus {
                        focus_style
                    } else {
                        default_style
                    },
                ))
                .block(
                    Block::default()
                        .title(title.as_str())
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(if is_focus {
                            focus_style
                        } else if *readonly {
                            readonly_style
                        } else {
                            default_style
                        }),
                );
                f.render_widget(widget, r);
            }
            FormItem::MultiSelect {
                name,
                selected,
                nullable,
                readonly,
                ..
            } => {
                let title = if *nullable {
                    name.clone()
                } else {
                    format!("{}(*)", name)
                };

                let widget = Paragraph::new(Span::styled(
                    selected.join(","),
                    if *readonly {
                        readonly_style
                    } else if is_focus {
                        focus_style
                    } else {
                        default_style
                    },
                ))
                .block(
                    Block::default()
                        .title(title.as_str())
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(if is_focus {
                            focus_style
                        } else if *readonly {
                            readonly_style
                        } else {
                            default_style
                        }),
                );
                f.render_widget(widget, r);
            }
            FormItem::Check {
                name,
                checked,
                readonly,
                ..
            } => {
                let widget = Paragraph::new(if *checked {
                    Span::raw("\u{2705}")
                } else {
                    Span::raw("\u{274E}")
                })
                .block(
                    Block::default()
                        .title(name.as_str())
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(if is_focus {
                            focus_style
                        } else if *readonly {
                            readonly_style
                        } else {
                            default_style
                        }),
                );
                f.render_widget(widget, r);
            }
            FormItem::Input {
                name,
                input,
                readonly,
                nullable,
                ..
            } => {
                let block = Block::default()
                    .title(if !*nullable {
                        format!("{}(*)", name)
                    } else {
                        name.to_string()
                    })
                    .borders(Borders::ALL)
                    .border_style(if is_focus {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default()
                    })
                    .border_type(BorderType::Rounded);

                if *readonly {
                    input.set_style(Style::default().fg(Color::DarkGray));
                } else {
                    input.set_style(Style::default());
                }
                input.set_block(block);
                f.render_widget(input.widget(), r);
            }
            FormItem::TextArea {
                name,
                textarea,
                nullable,
                can_null,
                is_null,
                readonly,
                ..
            } => f.render_widget(
                Paragraph::new(if !textarea.is_empty() {
                    textarea.lines().join("")
                } else if *can_null && *is_null {
                    String::from("(NULL)")
                } else {
                    String::new()
                })
                .block(
                    Block::default()
                        .title(if !*nullable {
                            format!("{}(*)", name.as_str())
                        } else {
                            String::from(name.as_str())
                        })
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                )
                .style(if is_focus {
                    focus_style
                } else if *readonly {
                    readonly_style
                } else {
                    default_style
                }),
                r,
            ),
            FormItem::List {
                name,
                items,
                nullable,
                readonly,
                ..
            } => {
                let title = if *nullable {
                    name.clone()
                } else {
                    format!("{}(*)", name)
                };
                let widget = Paragraph::new(Span::styled(
                    items.join(","),
                    if *readonly {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default()
                    },
                ))
                .block(
                    Block::default()
                        .title(title.as_str())
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(if is_focus {
                            focus_style
                        } else if *readonly {
                            readonly_style
                        } else {
                            default_style
                        }),
                );
                f.render_widget(widget, r);
            }
            FormItem::TableList {
                name,
                rows,
                nullable,
                show_fn,
                readonly,
                ..
            } => {
                let title = if *nullable {
                    name.clone()
                } else {
                    format!("{}(*)", name)
                };
                let widget = Paragraph::new(Span::styled(
                    rows.iter()
                        .map(|row| show_fn(row))
                        .collect::<Vec<String>>()
                        .join(","),
                    if *readonly {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default()
                    },
                ))
                .block(
                    Block::default()
                        .title(title.as_str())
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(if is_focus {
                            focus_style
                        } else if *readonly {
                            readonly_style
                        } else {
                            default_style
                        }),
                );
                f.render_widget(widget, r);
            }
        }
    }
    pub fn draw_dialog<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        match self {
            FormItem::Select {
                name,
                options,
                is_pop,
                state,
                ..
            } => {
                if *is_pop {
                    draw_select(f, name, options, state);
                }
            }
            FormItem::MultiSelect {
                name,
                is_pop,
                options,
                list_selected,
                state,
                ..
            } => {
                if *is_pop {
                    Self::draw_multi_select(f, name, options, list_selected, state);
                }
            }
            FormItem::List {
                name,
                items,
                dlg_state,
                state,
                textarea,
                ..
            } => match dlg_state {
                DialogState::List => Self::draw_list_list_dlg(f, name, items, state),
                DialogState::Input => Self::draw_list_input_dlg(f, name, textarea),
                DialogState::None => (),
            },
            FormItem::TableList {
                name,
                rows,
                columns,
                state,
                dlg_state,
                focus,
                offset,
                ..
            } => match dlg_state {
                DialogState::List => Self::draw_table_list_list_dlg(f, name, rows, columns, state),
                DialogState::Input => {
                    Self::draw_table_list_input_dlg(f, name, columns, focus, offset)
                }
                DialogState::None => (),
            },
            FormItem::TextArea {
                is_pop, textarea, ..
            } => {
                if *is_pop {
                    Self::draw_textarea_dlg(f, textarea);
                }
            }
            _ => (),
        }
    }

    fn draw_multi_select<B>(
        f: &mut Frame<B>,
        title: &str,
        options: &[String],
        selected: &[String],
        state: &mut ListState,
    ) where
        B: Backend,
    {
        let bounds = f.size();
        let width = min(bounds.width - 2, 40);
        let height = min(bounds.height, options.len() as u16 + 2);
        let left = (bounds.width - width) / 2;
        let top = (bounds.height - height) / 2;
        let rect = Rect::new(left, top, width, height);

        f.render_widget(Clear, rect);
        let items: Vec<ListItem> = options
            .iter()
            .map(|option| {
                let check = if selected.contains(option) {
                    "\u{2705}"
                } else {
                    "\u{274E}"
                };
                ListItem::new(Span::raw(format!("{} {}", check, option)))
            })
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
    fn draw_list_list_dlg<B>(
        f: &mut Frame<B>,
        name: &str,
        options: &[String],
        state: &mut ListState,
    ) where
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
                        .title(name)
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                )
                .highlight_style(Style::default().fg(Color::Green)),
            rect,
            state,
        );
    }
    fn draw_list_input_dlg<B>(f: &mut Frame<B>, name: &mut str, textarea: &mut TextArea)
    where
        B: Backend,
    {
        let bounds = f.size();
        let width = min(bounds.width - 2, 45);
        let height = min(bounds.height, 3);
        let left = (bounds.width - width) / 2;
        let top = (bounds.height - height) / 2;
        let rect = Rect::new(left, top, width, height);
        f.render_widget(Clear, rect);

        textarea.set_block(
            Block::default()
                .title(format!("Edit {}", name))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );
        f.render_widget(textarea.widget(), rect);
    }
    fn draw_table_list_list_dlg<B>(
        f: &mut Frame<B>,
        name: &str,
        rows: &[Vec<String>],
        columns: &mut Vec<ColumnInfo>,
        state: &mut TableState,
    ) where
        B: Backend,
    {
        let bounds = f.size();
        let width = min(bounds.width - 2, columns.len() as u16 * 15);
        let height = bounds.height / 2;
        let left = (bounds.width - width) / 2;
        let top = (bounds.height - height) / 2;
        let rect = Rect::new(left, top, width, height);

        f.render_widget(Clear, rect);

        let rows: Vec<Row> = rows.iter().map(|row| Row::new(row.clone())).collect();

        let cols = columns
            .iter()
            .map(|_| Constraint::Ratio(1, columns.len() as u32))
            .collect::<Vec<Constraint>>();

        let table = Table::new(rows)
            .header(Row::new(
                columns.iter().map(|c| c.name()).collect::<Vec<&str>>(),
            ))
            .block(
                Block::default()
                    .title(name)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .widths(&cols[..])
            .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(table, rect, state);
    }
    fn draw_table_list_input_dlg<B>(
        f: &mut Frame<B>,
        name: &str,
        columns: &mut Vec<ColumnInfo>,
        focus: &mut usize,
        offset: &mut usize,
    ) where
        B: Backend,
    {
        let bounds = f.size();
        let width = min(bounds.width - 2, 40);
        let height = (columns.len() * 3) as u16 + 2;
        let left = (bounds.width - width) / 2;
        let top = (bounds.height - height) / 2;
        let rect = Rect::new(left, top, width, height);

        f.render_widget(Clear, rect);

        f.render_widget(
            Block::default()
                .title(format!("Edit {}", name))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
            rect,
        );
        let inner = rect.inner(&Margin {
            horizontal: 1,
            vertical: 1,
        });
        let (start, end) = Self::get_items_bounds(columns, *focus, *offset, inner.height as usize);
        *offset = start;
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

        columns
            .iter_mut()
            .skip(*offset)
            .take(end - start)
            .enumerate()
            .for_each(|(i, item)| {
                item.draw(f, chunks[i], *focus - *offset == i);
            });

        columns
            .iter_mut()
            .skip(*offset)
            .take(end - start)
            .enumerate()
            .for_each(|(_i, item)| {
                item.draw_dialog(f);
            });
    }
    fn get_items_bounds(
        columns: &[ColumnInfo],
        focus: usize,
        offset: usize,
        max_height: usize,
    ) -> (usize, usize) {
        let offset = offset.min(columns.len().saturating_sub(1));
        let mut start = offset;
        let mut end = offset;
        let mut height = 0;
        for _item in columns.iter().skip(offset) {
            if height + 3 > max_height {
                break;
            }
            height += 3;
            end += 1;
        }
        let focus = focus.min(columns.len() - 1);
        while focus >= end {
            height = height.saturating_add(3);
            end += 1;
            while height > max_height {
                height = height.saturating_sub(3);
                start += 1;
            }
        }
        while focus < start {
            start -= 1;
            height = height.saturating_add(3);
            while height > max_height {
                end -= 1;
                height = height.saturating_sub(3);
            }
        }
        (start, end)
    }
    fn draw_textarea_dlg<B>(f: &mut Frame<B>, textarea: &mut TextArea<'a>)
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
        f.render_widget(textarea.widget(), rect);
    }
    pub fn handle_event(&mut self, key: &Key) -> Result<FormItemResult> {
        let result = match self {
            FormItem::Select {
                name,
                options,
                selected,
                nullable,
                is_pop,
                state,
                match_str,
                readonly,
                ..
            } => {
                if !*readonly {
                    handle_select_event(
                        name, options, selected, nullable, is_pop, state, match_str, key,
                    )
                } else {
                    FormItemResult::UnHandled
                }
            }
            FormItem::MultiSelect {
                selected,
                nullable,
                options,
                list_selected,
                is_pop,
                state,
                readonly,
                ..
            } => {
                if !*readonly {
                    Self::handle_multi_select_event(
                        options,
                        selected,
                        list_selected,
                        nullable,
                        is_pop,
                        state,
                        key,
                    )
                } else {
                    FormItemResult::UnHandled
                }
            }
            FormItem::Check {
                name,
                checked,
                readonly,
                ..
            } => {
                if !*readonly {
                    if matches!(key, &CONFIRM_KEY) {
                        *checked = !*checked;
                        FormItemResult::Changed(name.to_string(), checked.to_string())
                    } else {
                        FormItemResult::UnHandled
                    }
                } else {
                    FormItemResult::UnHandled
                }
            }
            FormItem::List {
                items,
                state,
                textarea,
                dlg_state,
                edit_index,
                readonly,
                ..
            } => {
                if !*readonly {
                    Self::handle_list_event(items, state, textarea, dlg_state, edit_index, key)
                } else {
                    FormItemResult::UnHandled
                }
            }
            FormItem::TableList {
                rows,
                columns,
                state,
                dlg_state,
                focus,
                offset,
                edit_index,
                readonly,
                ..
            } => {
                if !*readonly {
                    Self::handle_table_list_event(
                        rows, columns, state, dlg_state, focus, offset, edit_index, key,
                    )?
                } else {
                    FormItemResult::UnHandled
                }
            }
            FormItem::Input {
                input,
                readonly,
                can_null,
                is_null,
                ..
            } => match key {
                Key {
                    code: KeyCode::Enter,
                    ..
                }
                | Key {
                    code: KeyCode::Tab, ..
                }
                | Key {
                    code: KeyCode::Up, ..
                }
                | Key {
                    code: KeyCode::Down,
                    ..
                }
                | Key {
                    code: KeyCode::Esc, ..
                } => FormItemResult::UnHandled,
                &SET_NULL_KEY => {
                    if *can_null {
                        input.move_cursor(CursorMove::End);
                        input.delete_line_by_head();
                        input.insert_str("(NULL)");
                        *is_null = true;
                    }
                    FormItemResult::Handled
                }
                _ => {
                    if !*readonly {
                        let key: Input = key.to_owned().into();
                        if input.input(key.clone()) {
                            if *can_null && *is_null {
                                input.move_cursor(CursorMove::End);
                                input.delete_line_by_head();
                                if input.input(key) {
                                    *is_null = false;
                                }
                            }
                            FormItemResult::Handled
                        } else {
                            FormItemResult::UnHandled
                        }
                    } else {
                        FormItemResult::UnHandled
                    }
                }
            },
            FormItem::TextArea {
                textarea,
                is_pop,
                readonly,
                ..
            } => {
                if !*readonly {
                    if *is_pop {
                        match key {
                            &SAVE_KEY | &CANCEL_KEY => {
                                *is_pop = false;
                                FormItemResult::Handled
                            }
                            _ => {
                                let key: Input = key.to_owned().into();
                                if textarea.input(key) {
                                    FormItemResult::Handled
                                } else {
                                    FormItemResult::UnHandled
                                }
                            }
                        }
                    } else if matches!(key, &CONFIRM_KEY) {
                        *is_pop = true;
                        FormItemResult::Handled
                    } else {
                        FormItemResult::UnHandled
                    }
                } else {
                    FormItemResult::UnHandled
                }
            }
        };
        Ok(result)
    }
    pub fn get_commands(&self) -> Vec<Command> {
        match self {
            FormItem::Select { is_pop, .. } => {
                if *is_pop {
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
                } else {
                    vec![Command {
                        name: "Open Options",
                        key: CONFIRM_KEY,
                    }]
                }
            }
            FormItem::MultiSelect { is_pop, state, .. } => {
                if *is_pop {
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
                    if state.selected().is_some() {
                        cmds.push(Command {
                            name: "Toggle",
                            key: SPACE_KEY,
                        });
                    }
                    cmds.extend(vec![
                        Command {
                            name: "Cancel",
                            key: CANCEL_KEY,
                        },
                        Command {
                            name: "Ok",
                            key: CONFIRM_KEY,
                        },
                    ]);
                    cmds
                } else {
                    vec![Command {
                        name: "Open Options",
                        key: CONFIRM_KEY,
                    }]
                }
            }
            FormItem::TextArea { is_pop, .. } => {
                if *is_pop {
                    vec![
                        Command {
                            name: "Cancel",
                            key: CANCEL_KEY,
                        },
                        Command {
                            name: "Ok",
                            key: CONFIRM_KEY,
                        },
                    ]
                } else {
                    vec![Command {
                        name: "Open",
                        key: CONFIRM_KEY,
                    }]
                }
            }
            FormItem::List {
                dlg_state, state, ..
            } => match dlg_state {
                DialogState::List => {
                    let mut cmds = vec![
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
                            name: "New",
                            key: NEW_KEY,
                        },
                    ];
                    if state.selected().is_some() {
                        cmds.extend(vec![
                            Command {
                                name: "Edit",
                                key: CONFIRM_KEY,
                            },
                            Command {
                                name: "Delete",
                                key: DELETE_KEY,
                            },
                        ]);
                    }
                    cmds
                }
                DialogState::Input => {
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
                DialogState::None => {
                    vec![Command {
                        name: "Open List",
                        key: CONFIRM_KEY,
                    }]
                }
            },
            FormItem::TableList {
                dlg_state, state, ..
            } => match dlg_state {
                DialogState::List => {
                    let mut cmds = vec![
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
                            name: "New",
                            key: NEW_KEY,
                        },
                    ];
                    if state.selected().is_some() {
                        cmds.extend(vec![
                            Command {
                                name: "Delete",
                                key: DELETE_KEY,
                            },
                            Command {
                                name: "Edit",
                                key: CONFIRM_KEY,
                            },
                        ]);
                    }
                    cmds
                }
                DialogState::Input => {
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
                            name: "Save",
                            key: SAVE_KEY,
                        },
                    ]
                }
                DialogState::None => {
                    vec![Command {
                        name: "Open",
                        key: CONFIRM_KEY,
                    }]
                }
            },
            FormItem::Check { .. } => {
                vec![Command {
                    name: "Toggle",
                    key: CONFIRM_KEY,
                }]
            }
            FormItem::Input { can_null, .. } => {
                if *can_null {
                    vec![Command {
                        name: "Set Null",
                        key: SET_NULL_KEY,
                    }]
                } else {
                    vec![]
                }
            }
        }
    }
    fn handle_multi_select_event(
        options: &Vec<String>,
        selected: &mut Vec<String>,
        list_selected: &mut Vec<String>,
        nullable: &bool,
        is_pop: &mut bool,
        state: &mut ListState,
        key: &Key,
    ) -> FormItemResult {
        if *is_pop {
            match *key {
                UP_KEY => {
                    if !options.is_empty() {
                        let index = state.selected().unwrap_or_default();
                        let new_index = if index >= 1 { index - 1 } else { 0 };
                        state.select(Some(new_index));
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
                    }
                    FormItemResult::Handled
                }
                SPACE_KEY => {
                    if let Some(index) = state.selected() {
                        let option = &options[index];
                        let sel_index = list_selected.iter().position(|sel| *sel == *option);
                        if let Some(i) = sel_index {
                            list_selected.remove(i);
                        } else {
                            list_selected.push(option.to_string());
                        }
                    }
                    FormItemResult::Handled
                }
                CONFIRM_KEY => {
                    *selected = list_selected.to_vec();
                    list_selected.clear();
                    *is_pop = false;
                    FormItemResult::Handled
                }
                CANCEL_KEY => {
                    *is_pop = false;
                    FormItemResult::Handled
                }
                CLEAR_KEY => {
                    if *nullable {
                        selected.clear();
                        list_selected.clear();
                    }
                    FormItemResult::Handled
                }
                _ => FormItemResult::UnHandled,
            }
        } else {
            match *key {
                NEW_KEY => FormItemResult::Handled,
                CONFIRM_KEY => {
                    *is_pop = true;
                    *list_selected = selected.clone();
                    FormItemResult::Handled
                }
                _ => FormItemResult::UnHandled,
            }
        }
    }
    fn handle_list_event(
        items: &mut Vec<String>,
        state: &mut ListState,
        textarea: &mut TextArea<'a>,
        dlg_state: &mut DialogState,
        edit_index: &mut Option<usize>,
        key: &Key,
    ) -> FormItemResult {
        match dlg_state {
            DialogState::List => {
                match *key {
                    SAVE_KEY | CANCEL_KEY => {
                        *dlg_state = DialogState::None;
                    }
                    UP_KEY => {
                        if !items.is_empty() {
                            let index = state.selected().unwrap_or_default();
                            let new_index = if index >= 1 { index - 1 } else { 0 };
                            state.select(Some(new_index));
                        }
                    }
                    DOWN_KEY => {
                        if !items.is_empty() {
                            let index = if let Some(i) = state.selected() {
                                min(i + 1, items.len() - 1)
                            } else {
                                0
                            };
                            state.select(Some(index));
                        }
                    }
                    DELETE_KEY => {
                        if let Some(index) = state.selected() {
                            items.remove(index);
                            state.select(None);
                        }
                    }
                    NEW_KEY => {
                        *textarea = TextArea::default();
                        *dlg_state = DialogState::Input;
                    }
                    CONFIRM_KEY => {
                        if let Some(index) = state.selected() {
                            let item = &items[index];
                            *textarea = TextArea::from([item]);
                            *dlg_state = DialogState::Input;
                        }
                    }

                    _ => (),
                }
                FormItemResult::Handled
            }
            DialogState::Input => {
                match *key {
                    CANCEL_KEY => {
                        *dlg_state = DialogState::List;
                        *edit_index = None;
                    }
                    SAVE_KEY => {
                        let item = textarea.lines().join("\n");
                        if let Some(index) = edit_index {
                            items[*index] = item;
                            *edit_index = None;
                        } else {
                            items.push(item);
                        }
                        *dlg_state = DialogState::List;
                    }
                    Key {
                        code: KeyCode::Enter,
                        ..
                    }
                    | Key {
                        code: KeyCode::Tab, ..
                    }
                    | Key {
                        code: KeyCode::Up, ..
                    }
                    | Key {
                        code: KeyCode::Down,
                        ..
                    } => {}
                    _ => {
                        let key: Input = key.clone().into();
                        textarea.input(key);
                    }
                }
                FormItemResult::Handled
            }
            DialogState::None => match *key {
                CONFIRM_KEY => {
                    *dlg_state = DialogState::List;
                    FormItemResult::Handled
                }
                _ => FormItemResult::UnHandled,
            },
        }
    }
    fn handle_table_list_event(
        rows: &mut Vec<Vec<String>>,
        columns: &mut Vec<ColumnInfo>,
        state: &mut TableState,
        dlg_state: &mut DialogState,
        focus: &mut usize,
        offset: &mut usize,
        edit_index: &mut Option<usize>,
        key: &Key,
    ) -> Result<FormItemResult> {
        match dlg_state {
            DialogState::List => {
                match *key {
                    SAVE_KEY | CANCEL_KEY => {
                        *dlg_state = DialogState::None;
                    }
                    UP_KEY => {
                        if !rows.is_empty() {
                            let index = state.selected().unwrap_or_default();
                            let new_index = if index >= 1 { index - 1 } else { 0 };
                            state.select(Some(new_index));
                        }
                    }
                    DOWN_KEY => {
                        if !rows.is_empty() {
                            let index = if let Some(i) = state.selected() {
                                min(i + 1, rows.len() - 1)
                            } else {
                                0
                            };
                            state.select(Some(index));
                        }
                    }
                    DELETE_KEY => {
                        if let Some(index) = state.selected() {
                            rows.remove(index);
                            state.select(None);
                        }
                    }
                    NEW_KEY => {
                        columns.iter_mut().for_each(|column| match column {
                            ColumnInfo::Input { value, .. } => *value = String::default(),
                            ColumnInfo::Select { selected, .. } => *selected = None,
                        });
                        *dlg_state = DialogState::Input;
                    }
                    CONFIRM_KEY => {
                        if let Some(index) = state.selected() {
                            *edit_index = Some(index);
                            let row = &rows[index];
                            columns
                                .iter_mut()
                                .enumerate()
                                .for_each(|(i, column)| match column {
                                    ColumnInfo::Input { value, .. } => *value = row[i].clone(),
                                    ColumnInfo::Select { selected, .. } => {
                                        *selected = Some(row[i].clone())
                                    }
                                });
                            *dlg_state = DialogState::None;
                        }
                    }
                    _ => (),
                }
                Ok(FormItemResult::Handled)
            }
            DialogState::Input => {
                let result = columns[*focus].handle_event(key)?;
                match result {
                    FormItemResult::UnHandled => {
                        match *key {
                            UP_KEY => {
                                if *focus != 0 {
                                    *focus -= 1;
                                }
                            }
                            DOWN_KEY => {
                                if *focus < columns.len() - 1 {
                                    *focus += 1;
                                }
                            }
                            CANCEL_KEY => {
                                *dlg_state = DialogState::List;
                                *focus = 0;
                                *offset = 0;
                                *edit_index = None;
                            }
                            SAVE_KEY => {
                                Self::validate_input(columns)?;
                                let data = Self::get_data(columns);
                                if let Some(index) = edit_index {
                                    rows[*index] = data;
                                    *edit_index = None;
                                } else {
                                    rows.push(data);
                                }
                                *dlg_state = DialogState::List;
                            }
                            _ => (),
                        };

                        Ok(FormItemResult::Handled)
                    }
                    _ => Ok(result),
                }
            }
            DialogState::None => match *key {
                CONFIRM_KEY => {
                    *dlg_state = DialogState::List;
                    Ok(FormItemResult::Handled)
                }
                _ => Ok(FormItemResult::UnHandled),
            },
        }
    }
    fn validate_input(columns: &mut [ColumnInfo]) -> Result<()> {
        for item in columns.iter() {
            match &item {
                ColumnInfo::Input {
                    name,
                    value,
                    nullable,
                    ..
                } => {
                    if !nullable && value.is_empty() {
                        return Err(Error::msg(format!("Please input {}", name)));
                    }
                }
                ColumnInfo::Select {
                    name,
                    selected,
                    nullable,
                    ..
                } => {
                    if !nullable && selected.is_none() {
                        return Err(Error::msg(format!("Please select {}", name)));
                    }
                }
            }
        }
        Ok(())
    }
    pub fn get_data(columns: &[ColumnInfo]) -> Vec<String> {
        let mut data = vec![];
        for item in columns.iter() {
            match item {
                ColumnInfo::Input { value, .. } => {
                    data.push(value.clone());
                }
                ColumnInfo::Select { selected, .. } => {
                    data.push(selected.clone().unwrap_or_default());
                }
            }
        }
        data
    }
}
