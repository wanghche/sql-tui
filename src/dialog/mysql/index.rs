use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::mysql::{
        show_mysql_index_field, Field, Index, IndexKind, IndexMethod, IndexOrder, Version,
    },
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

pub struct IndexDialog<'a> {
    id: Option<Uuid>,
    form: Form<'a>,
}

impl<'a> IndexDialog<'a> {
    pub fn new(fields: &[Field], ver: &Version, index: Option<&Index>) -> Self {
        let mut form = Form::default();
        form.set_title("Edit Index".to_string());
        let mut columns = vec![
            ColumnInfo::Select {
                name: "field".to_string(),
                nullable: false,
                options: fields.iter().map(|f| f.name().to_string()).collect(),
                selected: None,
                state: ListState::default(),
                is_pop: false,
                match_str: String::new(),
            },
            ColumnInfo::Input {
                name: "sub_part".to_string(),
                nullable: true,
                value: String::default(),
            },
        ];
        if matches!(ver, Version::Eight) {
            columns.push(ColumnInfo::Select {
                name: "order".to_string(),
                options: IndexOrder::iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
                nullable: true,
                selected: None,
                state: ListState::default(),
                is_pop: false,
                match_str: String::new(),
            });
        }
        form.set_items(if let Some(i) = index {
            vec![
                FormItem::new_input("name".to_string(), Some(i.name()), false, false, false),
                FormItem::new_table_list(
                    "fields".to_string(),
                    i.fields()
                        .iter()
                        .map(|f| {
                            vec![
                                f.name.clone(),
                                f.sub_part.map(|s| s.to_string()).unwrap_or_default(),
                                f.order.clone().map(|o| o.to_string()).unwrap_or_default(),
                            ]
                        })
                        .collect(),
                    columns,
                    false,
                    show_mysql_index_field,
                    false,
                ),
                FormItem::new_select(
                    "kind".to_string(),
                    IndexKind::iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>(),
                    Some(i.kind().to_string()),
                    true,
                    false,
                ),
                FormItem::new_select(
                    "method".to_string(),
                    IndexMethod::iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>(),
                    i.method().map(|s| s.to_string()),
                    true,
                    false,
                ),
                FormItem::new_input("comment".to_string(), i.comment(), true, false, false),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_table_list(
                    "fields".to_string(),
                    vec![],
                    columns,
                    false,
                    show_mysql_index_field,
                    false,
                ),
                FormItem::new_select(
                    "kind".to_string(),
                    IndexKind::iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>(),
                    None,
                    true,
                    false,
                ),
                FormItem::new_select(
                    "method".to_string(),
                    IndexMethod::iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>(),
                    None,
                    true,
                    false,
                ),
                FormItem::new_input("comment".to_string(), None, true, false, false),
            ]
        });

        IndexDialog {
            id: index.map(|i| i.id().to_owned()),
            form,
        }
    }
    pub fn get_id(&self) -> Option<&Uuid> {
        self.id.as_ref()
    }
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
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
}
