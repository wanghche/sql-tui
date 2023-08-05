use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::pg::{show_pg_exclude_field, Connections, Exclude, Field, IndexMethod, Schema},
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

pub struct ExcludeDialog<'a> {
    id: Option<Uuid>,
    form: Form<'a>,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<PGPools>>,
    conn_id: Uuid,
}
impl<'a> ExcludeDialog<'a> {
    pub fn new(
        fields: &[Field],
        schemas: &[Schema],
        exclude: Option<&Exclude>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<PGPools>>,
        conn_id: &Uuid,
    ) -> Self {
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
            conns,
            pools,
            conn_id: conn_id.to_owned(),
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
                            } = &mut columns[2]
                            {
                                *options = operators
                                    .iter()
                                    .map(|r| r.try_get("opcname").unwrap())
                                    .collect();
                                *selected = None;
                            }
                        }
                    }
                    "operator schema" => {
                        let operators = fetch_pg_query(
                            self.conns.clone(),
                            self.pools.clone(),
                            &self.conn_id,
                            None,
                            format!(
                                "
                                select
                                    distinct oprname 
                                from
                                    pg_operator
                                where oprnamespace = '{}'::regnamespace::oid",
                                selected
                            )
                            .as_str(),
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
                                    .map(|r| r.try_get("oprname").unwrap())
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
