use crate::{
    app::{ComponentResult, DialogResult, MainPanel},
    component::{get_table_down_index, get_table_up_index, Command, CommandBarComponent},
    dialog::{
        confirm::{ConfirmDialog, Kind as ConfirmKind},
        DataDialog,
    },
    event::{config::*, Key},
    model::pg::{
        convert_show_column_to_pg_fields, get_pg_field_value, Connections, Field, FieldKind,
    },
    pool::{fetch_one_pg, fetch_pg_query, get_pg_pool, PGPools},
};
use anyhow::{Error, Result};
use sqlx::{postgres::PgRow, Postgres, QueryBuilder, Row};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use time::{macros::format_description, Date, Time};
use tui::{
    backend::Backend,
    layout::{Constraint, Margin, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Row as RowUI, Table as TableUI, TableState},
    Frame,
};
use uuid::Uuid;

pub struct DataListComponent<'a> {
    conn_id: Option<Uuid>,
    db_name: Option<String>,
    schema_name: Option<String>,
    table_name: Option<String>,
    state: TableState,
    rows: Vec<PgRow>,
    page: usize,
    page_size: usize,
    total_page: usize,
    fields: Vec<Field>,
    parent: Option<MainPanel>,
    create_dlg: Option<DataDialog<'a>>,
    edit_dlg: Option<DataDialog<'a>>,
    delete_dlg: Option<ConfirmDialog>,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<PGPools>>,
    cmd_bar: Rc<RefCell<CommandBarComponent>>,
}

impl<'a> DataListComponent<'a> {
    pub fn new(
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<PGPools>>,
        cmd_bar: Rc<RefCell<CommandBarComponent>>,
    ) -> Self {
        DataListComponent {
            conn_id: None,
            db_name: None,
            schema_name: None,
            table_name: None,
            state: TableState::default(),
            parent: None,
            create_dlg: None,
            edit_dlg: None,
            delete_dlg: None,
            page: 0,
            total_page: 0,
            page_size: 1000,
            rows: Vec::new(),
            fields: Vec::new(),
            conns,
            pools,
            cmd_bar,
        }
    }
    pub async fn set_data(
        &mut self,
        conn_id: &Uuid,
        db_name: &str,
        schema_name: &str,
        table_name: &str,
        parent: MainPanel,
    ) -> Result<()> {
        self.conn_id = Some(*conn_id);
        self.db_name = Some(db_name.to_string());
        self.schema_name = Some(schema_name.to_string());
        self.table_name = Some(table_name.to_string());
        self.parent = Some(parent);
        self.state = TableState::default();
        let fields = fetch_pg_query(
            self.conns.clone(),
            self.pools.clone(),
            self.conn_id.as_ref().unwrap(),
            None,
            &format!(
                "SELECT
                    col_description((table_schema||'.'||table_name)::regclass::oid, ordinal_position) as comment,
                    *
                FROM
                    information_schema.columns
                WHERE
                    table_schema = '{}' AND table_name = '{}'
                ORDER BY ordinal_position ASC",
                schema_name, table_name
            ),
       )
        .await?;
        let keys = fetch_pg_query(
            self.conns.clone(),
            self.pools.clone(),
            conn_id,
            None,
            &format!(
                "
                    SELECT
                        a.attname
                    FROM
                        pg_index i
                    JOIN pg_attribute a
                        ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
                    WHERE i.indrelid = '{}'::regclass AND i.indisprimary",
                table_name
            ),
        )
        .await?;

        self.fields = convert_show_column_to_pg_fields(
            fields,
            keys.iter()
                .map(|k| k.try_get::<String, _>("attname").unwrap())
                .collect::<Vec<String>>(),
        );
        self.page = 1;
        self.rows = fetch_pg_query(
            self.conns.clone(),
            self.pools.clone(),
            self.conn_id.as_ref().unwrap(),
            Some(db_name),
            &format!(
                "SELECT * FROM {} LIMIT {} OFFSET {}",
                table_name,
                self.page_size,
                (self.page - 1) * self.page_size,
            ),
        )
        .await?;
        let total_count: i64 = fetch_one_pg(
            self.conns.clone(),
            self.pools.clone(),
            self.conn_id.as_ref().unwrap(),
            Some(db_name),
            &format!("SELECT count(*) FROM {}", table_name),
        )
        .await?
        .try_get(0)
        .unwrap();
        self.total_page = (total_count as f64 / self.page_size as f64).ceil() as usize;

        Ok(())
    }
    pub fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect, is_focus: bool)
    where
        B: Backend,
    {
        f.render_widget(
            Block::default()
                .title(format!(
                    "{} ({}/{})",
                    self.table_name.as_ref().unwrap(),
                    self.page,
                    self.total_page,
                ))
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
        let columns = &self
            .fields
            .iter()
            .map(|_c| Constraint::Ratio(1, self.fields.len() as u32))
            .collect::<Vec<Constraint>>();

        let table = TableUI::new(
            self.rows
                .iter()
                .map(|r| {
                    let d = self
                        .fields
                        .iter()
                        .map(|field| get_pg_field_value(field, r))
                        .collect::<Vec<String>>();
                    RowUI::new(d)
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(
            self.fields
                .iter()
                .clone()
                .map(|field| field.name().to_string())
                .collect::<Vec<String>>(),
        ))
        .block(Block::default())
        .widths(&columns[..])
        .highlight_style(Style::default().fg(Color::Green));

        f.render_stateful_widget(
            table,
            r.inner(&Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.state,
        );
        if is_focus {
            self.update_commands();
        }
    }
    pub fn draw_dialog<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        if let Some(dlg) = self.delete_dlg.as_ref() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.create_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.edit_dlg.as_mut() {
            dlg.draw(f);
        }
    }
    pub async fn handle_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if self.delete_dlg.is_some() {
            self.handle_delete_dlg_event(key).await
        } else if self.create_dlg.is_some() {
            self.handle_create_dlg_event(key).await
        } else if self.edit_dlg.is_some() {
            self.handle_edit_dlg_event(key).await
        } else {
            self.handle_main_event(key).await
        }
    }
    async fn handle_delete_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.delete_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.state.selected() {
                        let row = &self.rows[index];
                        let keys: Vec<&Field> = self.fields.iter().filter(|c| c.key()).collect();
                        if !keys.is_empty() {
                            let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(format!(
                                "DELETE FROM {} WHERE ",
                                self.table_name.as_deref().unwrap()
                            ));
                            let mut sep = builder.separated(" AND ");
                            keys.iter().for_each(|k| match k.kind() {
                                FieldKind::Int2 | FieldKind::Serial2 | FieldKind::SmallSerial => {
                                    let value: i16 = row.try_get(k.name()).unwrap();
                                    sep.push_unseparated(format!("{}=", k.name()))
                                        .push_bind(value);
                                }
                                FieldKind::Int4 | FieldKind::Serial4 | FieldKind::Serial => {
                                    let value: i32 = row.try_get(k.name()).unwrap();
                                    sep.push_unseparated(format!("{}=", k.name()))
                                        .push_bind(value);
                                }
                                FieldKind::Int8 | FieldKind::Serial8 | FieldKind::BigSerial => {
                                    let value: i64 = row.try_get(k.name()).unwrap();
                                    sep.push_unseparated(format!("{}=", k.name()))
                                        .push_bind(value);
                                }
                                FieldKind::Numeric | FieldKind::Decimal | FieldKind::Float4 => {
                                    let value: f32 = row.try_get(k.name()).unwrap();
                                    sep.push_unseparated(format!("{}=", k.name()))
                                        .push_bind(value);
                                }
                                FieldKind::Float8 => {
                                    let value: f64 = row.try_get(k.name()).unwrap();
                                    sep.push_unseparated(format!("{}=", k.name()))
                                        .push_bind(value);
                                }
                                _ => {
                                    let value: String = row.try_get(k.name()).unwrap();
                                    sep.push_unseparated(format!("{}=", k.name()))
                                        .push_bind(value);
                                }
                            });
                            let pool = get_pg_pool(
                                self.conns.clone(),
                                self.pools.clone(),
                                &self.conn_id.unwrap(),
                                self.db_name.as_deref(),
                            )
                            .await?;
                            builder.build().execute(&pool).await?;
                        } else {
                            return Err(Error::msg("cannot delete this row"));
                        }

                        self.refresh().await?;
                    }
                    self.delete_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_create_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.create_dlg.as_mut() {
            match dlg.handle_event(key)? {
                DialogResult::Cancel => {
                    self.create_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    self.create_data(&map).await?;
                    self.refresh().await?;
                    self.create_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_edit_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.edit_dlg.as_mut() {
            match dlg.handle_event(key)? {
                DialogResult::Cancel => {
                    self.edit_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    if let Some(index) = self.state.selected() {
                        self.update_data(&map, &self.rows[index]).await?;
                        self.refresh().await?;
                        self.edit_dlg = None;
                    }
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_main_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            UP_KEY => {
                if !self.rows.is_empty() {
                    let index = get_table_up_index(self.state.selected());
                    self.state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.rows.is_empty() {
                    let index = get_table_down_index(self.state.selected(), self.rows.len());
                    self.state.select(Some(index));
                }
            }
            PAGE_NEXT_KEY => {
                if self.page < self.total_page {
                    self.page += 1;
                    self.refresh().await?;
                }
            }
            PAGE_PRIV_KEY => {
                if self.page > 1 {
                    self.page -= 1;
                    self.refresh().await?;
                }
            }
            BACK_KEY => {
                return Ok(ComponentResult::Back(self.parent.clone().unwrap()));
            }
            NEW_KEY => {
                let mut data_dlg =
                    DataDialog::new(format!("Add {}", self.table_name.as_ref().unwrap()));
                data_dlg.set_pg_fields_and_row(&self.fields, None, false);
                self.create_dlg = Some(data_dlg);
            }
            CONFIRM_KEY => {
                let key_count = self.fields.iter().filter(|c| c.key()).count();
                if key_count > 0 {
                    if let Some(index) = self.state.selected() {
                        let mut data_dlg =
                            DataDialog::new(format!("Edit {}", self.table_name.as_ref().unwrap()));
                        data_dlg.set_pg_fields_and_row(&self.fields, Some(&self.rows[index]), true);
                        self.edit_dlg = Some(data_dlg);
                    }
                }
            }
            DELETE_KEY => {
                self.delete_dlg = Some(ConfirmDialog::new(
                    ConfirmKind::Warning,
                    "Delete Data",
                    "Are you sure to delete this row?",
                ));
            }
            REFRESH_KEY => {
                self.refresh().await?;
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn create_data(&self, map: &HashMap<String, Option<String>>) -> Result<()> {
        let mut builder = QueryBuilder::new(format!(
            r#"INSERT INTO "{}"({}) "#,
            self.table_name.as_deref().unwrap(),
            map.iter()
                .map(|(key, _)| format!("\"{}\"", key))
                .collect::<Vec<String>>()
                .join(",")
        ));

        builder.push_values([map], |mut b, map| {
            map.iter().for_each(|(key, value)| {
                let field = self.fields.iter().find(|f| f.name() == key).unwrap();
                match field.kind() {
                    FieldKind::Int2 | FieldKind::Serial2 | FieldKind::SmallSerial => {
                        let v: Option<i16> = value.as_ref().map(|v| v.parse().unwrap());
                        b.push_bind(v);
                    }
                    FieldKind::Int4 | FieldKind::Serial4 | FieldKind::Serial => {
                        let v: Option<i32> = value.as_ref().map(|v| v.parse().unwrap());
                        b.push_bind(v);
                    }
                    FieldKind::Int8 | FieldKind::Serial8 | FieldKind::BigSerial => {
                        let v: Option<i64> = value.as_ref().map(|v| v.parse().unwrap());
                        b.push_bind(v);
                    }
                    FieldKind::Numeric | FieldKind::Decimal | FieldKind::Float4 => {
                        let v: Option<f32> = value.as_ref().map(|v| v.parse().unwrap());
                        b.push_bind(v);
                    }
                    FieldKind::Float8 => {
                        let v: Option<f64> = value.as_ref().map(|v| v.parse().unwrap());
                        b.push_bind(v);
                    }
                    FieldKind::Date => {
                        let v: Option<Date> = value.as_ref().map(|v| {
                            Date::parse(v, format_description!("[year]-[month]-[day]")).unwrap()
                        });
                        b.push_bind(v);
                    }
                    FieldKind::Time => {
                        let v: Option<Time> = value.as_ref().map(|v| {
                            Time::parse(v, format_description!("[hour]:[minute]:[second]")).unwrap()
                        });
                        b.push_bind(v);
                    }
                    _ => {
                        b.push_bind(value);
                    }
                }
            });
        });

        let pool = get_pg_pool(
            self.conns.clone(),
            self.pools.clone(),
            &self.conn_id.unwrap(),
            self.db_name.as_deref(),
        )
        .await?;

        builder.build().execute(&pool).await?;

        Ok(())
    }
    async fn update_data(&self, map: &HashMap<String, Option<String>>, row: &PgRow) -> Result<()> {
        let keys: Vec<&Field> = self.fields.iter().filter(|c| c.key()).collect();
        let mut builder: QueryBuilder<Postgres> =
            QueryBuilder::new(format!("UPDATE {} SET ", self.table_name.as_ref().unwrap()));

        let mut sep = builder.separated(", ");

        map.iter().for_each(|(key, value)| {
            let field = self.fields.iter().find(|f| !f.key() && f.name() == key);
            if let Some(field) = field {
                match field.kind() {
                    FieldKind::Int2 | FieldKind::Serial2 | FieldKind::SmallSerial => {
                        let v: Option<i16> = value.as_ref().map(|v| v.parse().unwrap());
                        sep.push_unseparated(format!("\"{}\"=", key)).push_bind(v);
                    }
                    FieldKind::Int4 | FieldKind::Serial4 | FieldKind::Serial => {
                        let v: Option<i32> = value.as_ref().map(|v| v.parse().unwrap());
                        sep.push_unseparated(format!("\"{}\"=", key)).push_bind(v);
                    }
                    FieldKind::Int8 | FieldKind::Serial8 | FieldKind::BigSerial => {
                        let v: Option<i64> = value.as_ref().map(|v| v.parse().unwrap());
                        sep.push_unseparated(format!("\"{}\"=", key)).push_bind(v);
                    }
                    FieldKind::Numeric | FieldKind::Decimal | FieldKind::Float4 => {
                        let v: Option<f32> = value.as_ref().map(|v| v.parse().unwrap());
                        sep.push_unseparated(format!("\"{}\"=", key)).push_bind(v);
                    }
                    FieldKind::Float8 => {
                        let v: Option<f64> = value.as_ref().map(|v| v.parse().unwrap());
                        sep.push_unseparated(format!("\"{}\"=", key)).push_bind(v);
                    }
                    FieldKind::Date => {
                        let v: Option<Date> = value.as_ref().map(|v| {
                            Date::parse(v, format_description!("[year]-[month]-[day]")).unwrap()
                        });
                        sep.push_unseparated(format!("\"{}\"=", key)).push_bind(v);
                    }
                    FieldKind::Time => {
                        let v: Option<Time> = value.as_ref().map(|v| {
                            Time::parse(v, format_description!("[hour]:[minute]:[second]")).unwrap()
                        });
                        sep.push_unseparated(format!("\"{}\"=", key)).push_bind(v);
                    }
                    _ => {
                        sep.push_unseparated(format!("\"{}\"=", key))
                            .push_bind(value);
                    }
                }
            }
        });
        builder.push(" WHERE ");
        let mut sep = builder.separated(" AND ");
        keys.iter().for_each(|field| match field.kind() {
            FieldKind::Int2 | FieldKind::Serial2 | FieldKind::SmallSerial => {
                let v: Option<i16> = row.try_get(field.name()).unwrap();
                sep.push_unseparated(format!("\"{}\"=", field.name()))
                    .push_bind(v);
            }
            FieldKind::Int4 | FieldKind::Serial4 | FieldKind::Serial => {
                let v: Option<i32> = row.try_get(field.name()).unwrap();
                sep.push_unseparated(format!("\"{}\"=", field.name()))
                    .push_bind(v);
            }
            FieldKind::Int8 | FieldKind::Serial8 | FieldKind::BigSerial => {
                let v: Option<i64> = row.try_get(field.name()).unwrap();
                sep.push_unseparated(format!("\"{}\"=", field.name()))
                    .push_bind(v);
            }
            FieldKind::Numeric | FieldKind::Decimal | FieldKind::Float4 => {
                let v: Option<f32> = row.try_get(field.name()).unwrap();
                sep.push_unseparated(format!("\"{}\"=", field.name()))
                    .push_bind(v);
            }
            FieldKind::Float8 => {
                let v: Option<f64> = row.try_get(field.name()).unwrap();
                sep.push_unseparated(format!("\"{}\"=", field.name()))
                    .push_bind(v);
            }
            FieldKind::Date => {
                let v: Option<Date> = row.try_get(field.name()).unwrap();
                sep.push_unseparated(format!("\"{}\"=", field.name()))
                    .push_bind(v);
            }
            FieldKind::Time => {
                let v: Option<Time> = row.try_get(field.name()).unwrap();
                sep.push_unseparated(format!("\"{}\"=", field.name()))
                    .push_bind(v);
            }
            _ => {
                sep.push_unseparated(format!("\"{}\"=", field.name()))
                    .push_bind(row.try_get::<String, _>(field.name()).unwrap());
            }
        });

        let pool = get_pg_pool(
            self.conns.clone(),
            self.pools.clone(),
            &self.conn_id.unwrap(),
            self.db_name.as_deref(),
        )
        .await?;
        builder.build().execute(&pool).await?;
        Ok(())
    }
    pub async fn refresh(&mut self) -> Result<()> {
        self.rows = fetch_pg_query(
            self.conns.clone(),
            self.pools.clone(),
            self.conn_id.as_ref().unwrap(),
            self.db_name.as_deref(),
            &format!(
                "SELECT * FROM {} LIMIT {} OFFSET {}",
                self.table_name.as_ref().unwrap(),
                self.page_size,
                (self.page - 1) * self.page_size,
            ),
        )
        .await?;
        let total_count: i64 = fetch_one_pg(
            self.conns.clone(),
            self.pools.clone(),
            self.conn_id.as_ref().unwrap(),
            self.db_name.as_deref(),
            &format!("SELECT count(*) FROM {}", self.table_name.as_ref().unwrap(),),
        )
        .await?
        .try_get(0)
        .unwrap();
        self.total_page = (total_count as f64 / self.page_size as f64).ceil() as usize;
        Ok(())
    }
    fn update_commands(&mut self) {
        let mut cmds = if let Some(dlg) = self.delete_dlg.as_mut() {
            dlg.get_commands()
        } else if let Some(dlg) = self.create_dlg.as_mut() {
            dlg.get_commands()
        } else if let Some(dlg) = self.edit_dlg.as_mut() {
            dlg.get_commands()
        } else {
            self.get_main_commands()
        };
        self.cmd_bar.borrow_mut().set_commands(&mut cmds);
    }
    fn get_main_commands(&self) -> Vec<Command> {
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
                name: "Page Next",
                key: PAGE_NEXT_KEY,
            },
            Command {
                name: "Page Previous",
                key: PAGE_PRIV_KEY,
            },
            Command {
                name: "New Data",
                key: NEW_KEY,
            },
        ];
        let key_count = self.fields.iter().filter(|c| c.key()).count();
        if key_count > 0 && self.state.selected().is_some() {
            cmds.append(&mut vec![
                Command {
                    name: "Open Data",
                    key: CONFIRM_KEY,
                },
                Command {
                    name: "Delete Data",
                    key: DELETE_KEY,
                },
            ]);
        }
        cmds.push(Command {
            name: "Refresh",
            key: REFRESH_KEY,
        });
        cmds.push(Command {
            name: "Back",
            key: BACK_KEY,
        });
        cmds
    }
}
