use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::pg::{show_pg_exclude_field, Exclude, Field, IndexMethod, Schema},
    widget::{ColumnInfo, Form, FormItem},
};
use anyhow::Result;
use std::{cmp::min, collections::HashMap};
use strum::IntoEnumIterator;
use tui::{
    backend::Backend,
    layout::Rect,
    widgets::{Clear, ListState},
    Frame,
};
use uuid::Uuid;

pub struct ExcludeDialog<'a> {
    id: Option<Uuid>,
    form: Form<'a>,
}
impl<'a> ExcludeDialog<'a> {
    pub fn new(fields: &[Field], schemas: &[Schema], exclude: Option<&Exclude>) -> Self {
        let mut form = Form::default();
        form.set_items(if let Some(e) = exclude {
            vec![
                FormItem::new_input("name".to_string(), Some(e.name()), false, false, false),
                FormItem::new_select(
                    "index method".to_string(),
                    IndexMethod::iter().map(|s| s.to_string()).collect(),
                    e.index_method().map(|s| s.to_string()),
                    true,
                    false,
                ),
                FormItem::new_table_list(
                    "element".to_string(),
                    e.element()
                        .iter()
                        .map(|e| {
                            vec![
                                e.element().to_string(),
                                e.operator_class_schema()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default(),
                                e.operator_class()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default(),
                                e.operator_schema()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default(),
                                e.operator().map(|s| s.to_string()).unwrap_or_default(),
                            ]
                        })
                        .collect::<Vec<Vec<String>>>(),
                    vec![
                        ColumnInfo::Select {
                            name: "element".to_string(),
                            nullable: false,
                            options: fields.iter().map(|f| f.name().to_string()).collect(),
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "operator class schema".to_string(),
                            nullable: false,
                            options: schemas.iter().map(|s| s.name().to_string()).collect(),
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "operator class".to_string(),
                            nullable: false,
                            options: vec![],
                            selected: None,
                            state: ListState::default(),
                            match_str: String::new(),
                            is_pop: false,
                        },
                        ColumnInfo::Select {
                            name: "operator schema".to_string(),
                            nullable: false,
                            options: schemas.iter().map(|s| s.name().to_string()).collect(),
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "operator".to_string(),
                            nullable: false,
                            options: vec![],
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                    ],
                    false,
                    show_pg_exclude_field,
                    false,
                ),
                FormItem::new_input("comment".to_string(), None, true, false, false),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_select(
                    "index method".to_string(),
                    IndexMethod::iter().map(|s| s.to_string()).collect(),
                    None,
                    true,
                    false,
                ),
                FormItem::new_table_list(
                    "element".to_string(),
                    vec![],
                    vec![
                        ColumnInfo::Select {
                            name: "element".to_string(),
                            nullable: false,
                            options: fields.iter().map(|f| f.name().to_string()).collect(),
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "operator class schema".to_string(),
                            nullable: false,
                            options: schemas.iter().map(|s| s.name().to_string()).collect(),
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "operator class".to_string(),
                            nullable: false,
                            options: vec![],
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "operator schema".to_string(),
                            nullable: false,
                            options: schemas.iter().map(|s| s.name().to_string()).collect(),
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "operator".to_string(),
                            nullable: false,
                            options: vec![],
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                    ],
                    false,
                    show_pg_exclude_field,
                    false,
                ),
                FormItem::new_input("comment".to_string(), None, true, false, false),
            ]
        });
        ExcludeDialog {
            id: exclude.map(|e| e.id),
            form,
        }
    }
    pub fn get_id(&self) -> Option<&Uuid> {
        self.id.as_ref()
    }
    pub fn draw<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        let bounds = f.size();
        let width = min(bounds.width - 2, 60);

        let height = min(self.form.height(), bounds.height);
        let left = (bounds.width - width) / 2;
        let top = (bounds.height - height) / 2;
        let rect = Rect::new(left, top, width, height);
        f.render_widget(Clear, rect);

        self.form.draw(f, rect);
    }
    pub fn handle_event(
        &mut self,
        key: &Key,
    ) -> Result<DialogResult<HashMap<String, Option<String>>>> {
        let r = self.form.handle_event(key)?;
        match r {
            DialogResult::Confirm(mut map) => {
                if let Some(id) = self.id.as_ref() {
                    map.insert("id".to_string(), Some(id.to_string()));
                }
                Ok(DialogResult::Confirm(map))
            }
            _ => Ok(r),
        }
    }
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
}
