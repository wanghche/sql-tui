use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::pg::{show_pg_index_field, Connections, Field, Index, IndexMethod, Schema},
    pool::{fetch_pg_query, PGPools},
    widget::{ColumnInfo, Form, FormItem},
};
use anyhow::Result;
use sqlx::Row;
use std::{cell::RefCell, cmp::min, collections::HashMap, rc::Rc};
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
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<PGPools>>,
    conn_id: Uuid,
}

impl<'a> IndexDialog<'a> {
    pub fn new(
        fields: &Vec<Field>,
        schemas: &Vec<Schema>,
        index: Option<&Index>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<PGPools>>,
        conn_id: &Uuid,
    ) -> Self {
        let mut form = Form::default();
        form.set_title("Edit Index".to_string());
        form.set_items(if let Some(i) = index {
            vec![
                FormItem::new_input("name".to_string(), Some(i.name()), false, false, false),
                FormItem::new_table_list(
                    "fields".to_string(),
                    i.fields()
                        .iter()
                        .map(|f| {
                            let mut v = Vec::new();
                            v.push(f.name().to_string());
                            v.push(
                                f.collation_schema()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default(),
                            );
                            v.push(f.collation().map(|s| s.to_string()).unwrap_or_default());
                            v.push(
                                f.operator_class_schema()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default(),
                            );
                            v.push(
                                f.operator_class()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default(),
                            );
                            v.push(f.sort_order().map(|s| s.to_string()).unwrap_or_default());
                            v.push(f.nulls_order().map(|s| s.to_string()).unwrap_or_default());
                            v
                        })
                        .collect::<Vec<Vec<String>>>(),
                    vec![
                        ColumnInfo::Select {
                            name: "name".to_string(),
                            nullable: false,
                            options: fields.iter().map(|f| f.name().to_string()).collect(),
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "collation schema".to_string(),
                            nullable: true,
                            options: schemas.iter().map(|s| s.name().to_string()).collect(),
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "collation".to_string(),
                            nullable: true,
                            options: vec![],
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "operator class schema".to_string(),
                            nullable: true,
                            options: schemas.iter().map(|s| s.name().to_string()).collect(),
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "operator class".to_string(),
                            nullable: true,
                            options: vec![],
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "sort order".to_string(),
                            nullable: true,
                            options: vec!["ASC".to_string(), "DESC".to_string()],
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "nulls order".to_string(),
                            nullable: true,
                            options: vec!["NULLS FIRST".to_string(), "NULLS LAST".to_string()],
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                    ],
                    false,
                    show_pg_index_field,
                    true,
                ),
                FormItem::new_select(
                    "index method".to_string(),
                    IndexMethod::iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>(),
                    i.index_method().map(|m| m.to_string()),
                    true,
                    true,
                ),
                FormItem::new_check("unique".to_string(), i.unique(), true),
                FormItem::new_check("concurrent".to_string(), i.concurrent(), true),
                FormItem::new_input("comment".to_string(), i.comment(), true, false, false),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_table_list(
                    "fields".to_string(),
                    vec![],
                    vec![
                        ColumnInfo::Select {
                            name: "name".to_string(),
                            nullable: false,
                            options: fields.iter().map(|f| f.name().to_string()).collect(),
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "collation schema".to_string(),
                            nullable: true,
                            options: schemas.iter().map(|s| s.name().to_string()).collect(),
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "collation".to_string(),
                            nullable: true,
                            options: vec![],
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "operator class schema".to_string(),
                            nullable: true,
                            options: schemas.iter().map(|s| s.name().to_string()).collect(),
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "operator class".to_string(),
                            nullable: true,
                            options: vec![],
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "sort order".to_string(),
                            nullable: true,
                            options: vec!["ASC".to_string(), "DESC".to_string()],
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                        ColumnInfo::Select {
                            name: "nulls order".to_string(),
                            nullable: true,
                            options: vec!["NULLS FIRST".to_string(), "NULLS LAST".to_string()],
                            selected: None,
                            state: ListState::default(),
                            is_pop: false,
                            match_str: String::new(),
                        },
                    ],
                    false,
                    show_pg_index_field,
                    false,
                ),
                FormItem::new_select(
                    "index method".to_string(),
                    IndexMethod::iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>(),
                    None,
                    true,
                    false,
                ),
                FormItem::new_check("unique".to_string(), false, false),
                FormItem::new_check("concurrent".to_string(), false, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
            ]
        });

        IndexDialog {
            id: index.map(|i| i.id.clone()),
            form,
            conns,
            pools,
            conn_id: conn_id.to_owned(),
        }
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
        let rect = Rect::new(left, top, width, height as u16);
        f.render_widget(Clear, rect);

        self.form.draw(f, rect);
    }
    pub fn get_id(&self) -> Option<&Uuid> {
        self.id.as_ref()
    }
    pub async fn handle_event(
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
            DialogResult::Changed(name, selected) => {
                match name.as_str() {
                    "collation schema" => {
                        let collations = fetch_pg_query(
                            self.conns.clone(),
                            self.pools.clone(),
                            &self.conn_id,
                            None,
                            format!("select collname from pg_collation where collnamespace = '{}'::regnamespace::oid",selected).as_str() 
                        )
                        .await?;
                        if let FormItem::TableList { columns, .. } =
                            self.form.get_focus_item_mut().unwrap()
                        {
                            if let ColumnInfo::Select {
                                options, selected, ..
                            } = &mut columns[2]
                            {
                                *options = collations
                                    .iter()
                                    .map(|r| r.try_get("collname").unwrap())
                                    .collect();
                                *selected = None;
                            }
                        }
                    }
                    "operator class schema" => {
                        let operators = fetch_pg_query(
                            self.conns.clone(),
                            self.pools.clone(),
                            &self.conn_id,
                            None,
                            format!("select distinct opcname from pg_opclass where opcnamespace = '{}'::regnamespace::oid", selected).as_str(),
                        )
                        .await?;
                        if let FormItem::TableList { columns, .. } =
                            self.form.get_focus_item_mut().unwrap()
                        {
                            if let ColumnInfo::Select {
                                options, selected, ..
                            } = &mut columns[4]
                            {
                                *options = operators
                                    .iter()
                                    .map(|r| r.try_get("opcname").unwrap())
                                    .collect();
                                *selected = None;
                            }
                        }
                    }
                    _ => (),
                }
                Ok(DialogResult::Done)
            }
            _ => Ok(r),
        }
    }
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
}
