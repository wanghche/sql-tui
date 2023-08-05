use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::{
        mysql::{Connections as MySQLConnections, Database as MySQLDatabase},
        pg::{
            get_pg_db_names, get_pg_role_names, get_pg_table_spaces, Connections as PGConnections,
            Database as PGDatabase,
        },
        DatabaseKind, DB,
    },
    pool::{fetch_mysql_query, get_pg_pool, MySQLPools, PGPools},
    widget::{Form, FormItem},
};

use anyhow::Result;
use sqlx::Row;
use std::{cell::RefCell, cmp::min, collections::HashMap, rc::Rc};
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};
use uuid::Uuid;

#[derive(Default)]
pub enum Mode {
    #[default]
    Create,
    Edit,
}

#[derive(Default)]
pub struct DatabaseDialog<'a> {
    mode: Mode,
    conn_id: Uuid,
    kind: DatabaseKind,
    form: Form<'a>,
    mysql_pools: Rc<RefCell<MySQLPools>>,
    mysql_conns: Rc<RefCell<MySQLConnections>>,
    pg_pools: Rc<RefCell<PGPools>>,
    pg_conns: Rc<RefCell<PGConnections>>,
}

impl<'a> DatabaseDialog<'a> {
    pub async fn set_mysql_db(
        &mut self,
        mysql_conns: Rc<RefCell<MySQLConnections>>,
        mysql_pools: Rc<RefCell<MySQLPools>>,
        conn_id: &Uuid,
        db: Option<&MySQLDatabase>,
    ) -> Result<()> {
        self.mode = if db.is_some() {
            Mode::Edit
        } else {
            Mode::Create
        };
        self.mysql_conns = mysql_conns;
        self.mysql_pools = mysql_pools;
        self.kind = DatabaseKind::MySQL;
        self.conn_id = *conn_id;
        self.form = self.create_mysql_form(db).await?;
        Ok(())
    }
    pub async fn set_pg_db(
        &mut self,
        pg_conns: Rc<RefCell<PGConnections>>,
        pg_pools: Rc<RefCell<PGPools>>,
        conn_id: &Uuid,
        db: Option<&PGDatabase>,
    ) -> Result<()> {
        self.mode = if db.is_some() {
            Mode::Edit
        } else {
            Mode::Create
        };

        self.pg_conns = pg_conns;
        self.pg_pools = pg_pools;
        self.kind = DatabaseKind::PostgreSQL;
        self.conn_id = *conn_id;
        self.form = self.create_pg_form(db).await?;

        Ok(())
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
        let result = self.form.handle_event(key)?;
        match result {
            DialogResult::Changed(name, selected) => {
                if name == "character set" {
                    let rows = fetch_mysql_query(
                        self.mysql_conns.clone(),
                        self.mysql_pools.clone(),
                        &self.conn_id,
                        None,
                        format!("SHOW COLLATION WHERE Charset='{}'", selected).as_str(),
                    )
                    .await?;
                    self.form.set_item(
                        "collation",
                        FormItem::new_select(
                            "collation".to_string(),
                            rows.iter()
                                .map(|row| row.try_get("Collation").unwrap())
                                .collect(),
                            None,
                            true,
                            false,
                        ),
                    );
                }
                Ok(DialogResult::Done)
            }
            DialogResult::Confirm(mut map) => {
                map.insert("kind".to_string(), Some(self.kind.to_string()));
                map.insert("conn_id".to_string(), Some(self.conn_id.to_string()));
                Ok(DialogResult::Confirm(map))
            }
            DialogResult::Cancel => Ok(DialogResult::Cancel),
            _ => Ok(DialogResult::Done),
        }
    }
    pub fn get_mode(&self) -> &Mode {
        &self.mode
    }
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
    async fn create_mysql_form(&mut self, db: Option<&MySQLDatabase>) -> Result<Form<'a>> {
        let mut form = Form::default();
        form.set_title(if let Some(db) = db {
            db.name().to_string()
        } else {
            "New Database".to_string()
        });
        let charsets = fetch_mysql_query(
            self.mysql_conns.clone(),
            self.mysql_pools.clone(),
            &self.conn_id,
            None,
            "SHOW CHARACTER SET",
        )
        .await?
        .iter()
        .map(|row| row.try_get("Charset").unwrap())
        .collect();

        let items = if let Some(db) = db {
            let collations: Vec<String> = if let Some(charset) = db.character_set() {
                let rows = fetch_mysql_query(
                    self.mysql_conns.clone(),
                    self.mysql_pools.clone(),
                    &self.conn_id,
                    None,
                    format!("show collation where Charset='{}'", charset).as_str(),
                )
                .await?;
                rows.iter()
                    .map(|row| row.try_get("Collation").unwrap())
                    .collect()
            } else {
                vec![]
            };
            vec![
                FormItem::new_input("name".to_string(), Some(db.name()), false, false, true),
                FormItem::new_select(
                    "character set".to_string(),
                    charsets,
                    db.character_set().map(|s| s.to_string()),
                    true,
                    false,
                ),
                FormItem::new_select(
                    "collation".to_string(),
                    collations,
                    db.collation().map(|s| s.to_string()),
                    true,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_select("character set".to_string(), charsets, None, true, false),
                FormItem::new_select("collation".to_string(), vec![], None, true, false),
            ]
        };
        form.set_items(items);
        Ok(form)
    }
    async fn create_pg_form(&mut self, db: Option<&PGDatabase>) -> Result<Form<'a>> {
        let mut form = Form::default();
        form.set_title(if let Some(db) = db {
            db.name().to_string()
        } else {
            "New Database".to_string()
        });
        let pool = get_pg_pool(
            self.pg_conns.clone(),
            self.pg_pools.clone(),
            &self.conn_id,
            None,
        )
        .await?;

        let roles = get_pg_role_names(&pool).await?;
        let dbs = get_pg_db_names(&pool).await?;
        let tb_spaces = get_pg_table_spaces(&pool).await?;

        form.set_items(if let Some(db) = db {
            vec![
                FormItem::new_input("name".to_string(), Some(db.name()), false, false, false),
                FormItem::new_select(
                    "owner".to_string(),
                    roles,
                    db.get_owner().map(|s| s.to_string()),
                    true,
                    false,
                ),
                FormItem::new_input(
                    "collation order".to_string(),
                    db.get_collation(),
                    true,
                    false,
                    true,
                ),
                FormItem::new_input(
                    "character class".to_string(),
                    db.get_character_set(),
                    true,
                    false,
                    true,
                ),
                FormItem::new_select(
                    "template".to_string(),
                    dbs,
                    db.get_template().map(|s| s.to_string()),
                    true,
                    true,
                ),
                FormItem::new_select(
                    "tablespace".to_string(),
                    tb_spaces,
                    db.get_table_space().map(|s| s.to_string()),
                    true,
                    false,
                ),
                FormItem::new_input(
                    "connection limit".to_string(),
                    db.get_conn_limit(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_check("allow connection".to_string(), db.get_allow_conn(), false),
                FormItem::new_check("is template".to_string(), db.get_is_template(), false),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_select("owner".to_string(), roles, None, true, false),
                FormItem::new_input("collation order".to_string(), None, true, false, false),
                FormItem::new_input("character class".to_string(), None, true, false, false),
                FormItem::new_select("template".to_string(), dbs, None, true, false),
                FormItem::new_select("tablespace".to_string(), tb_spaces, None, true, false),
                FormItem::new_input(
                    "connection limit".to_string(),
                    Some("-1"),
                    true,
                    false,
                    false,
                ),
                FormItem::new_check("allow connection".to_string(), true, false),
                FormItem::new_check("is template".to_string(), true, false),
            ]
        });
        Ok(form)
    }
}
