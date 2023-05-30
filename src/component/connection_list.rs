use crate::{
    app::{ComponentResult, DialogResult, Focus, Goto},
    component::{get_table_down_index, get_table_up_index, Command, CommandBarComponent},
    config::Config,
    dialog::{
        confirm::{ConfirmDialog, Kind as ConfirmKind},
        database::{DatabaseDialog, Mode as DatabaseMode},
        schema::{Mode as SchemaMode, SchemaDialog},
        ConnectionDialog,
    },
    event::{config::*, Key},
    model::{
        get_all_connections,
        mysql::{
            delete_mysql_connection, get_mysql_connection, get_mysql_database, get_mysql_databases,
            save_mysql_connection, Connection as MySQLConnection, Connections as MySQLConnections,
            Database as MySQLDatabase,
        },
        pg::{
            delete_pg_connection, get_pg_connection, get_pg_database, get_pg_databases,
            get_pg_role_names, get_pg_schema, get_pg_schemas, save_pg_connection,
            Connection as PGConnection, Connections as PGConnections, Database as PGDatabase,
            Schema,
        },
        Connect, DatabaseKind, DB,
    },
    pool::{
        close_mysql_pool, close_pg_pool, execute_mysql_query, execute_mysql_query_unprepared,
        execute_pg_query, execute_pg_query_unprepared, get_mysql_pool, get_pg_pool, MySQLPools,
        PGPools,
    },
    widget::Select,
};
use anyhow::{Error, Result};
use chrono::Utc;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use strum::IntoEnumIterator;
use tui::{
    backend::Backend,
    layout::{Margin, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
    Frame,
};
use uuid::Uuid;

#[derive(Clone)]
enum TreeItem {
    Connection(ConnectionItem),
    Database(DatabaseItem),
    Schema(SchemaItem),
    Query(DatabaseSubItem),
    Table(DatabaseSubItem),
    View(DatabaseSubItem),
}
#[derive(Clone)]
struct ConnectionItem {
    pub id: Uuid,
    pub name: String,
    pub kind: DatabaseKind,
    pub is_collapsed: bool,
    pub is_open: bool,
}
#[derive(Clone)]
struct DatabaseItem {
    pub id: Uuid,
    pub conn_id: Uuid,
    pub name: String,
    pub kind: DatabaseKind,
    pub is_collapsed: bool,
    pub is_conn_collapsed: bool,
    pub is_open: bool,
}
#[derive(Clone)]
struct SchemaItem {
    pub id: Uuid,
    pub conn_id: Uuid,
    pub db_id: Uuid,
    pub db_name: String,
    pub name: String,
    pub is_collapsed: bool,
    pub is_db_collapsed: bool,
}
#[derive(Clone)]
struct DatabaseSubItem {
    conn_id: Uuid,
    db_id: Uuid,
    schema_id: Option<Uuid>,
    db_name: String,
    schema_name: Option<String>,
    kind: DatabaseKind,
    is_parent_collapsed: bool,
}

pub struct ConnectionListComponent<'a> {
    state: ListState,
    tree_items: Vec<TreeItem>,
    show_items: Vec<TreeItem>,
    new_select: Option<Select>,
    delete_conn_dlg: Option<ConfirmDialog>,
    delete_db_dlg: Option<ConfirmDialog>,
    delete_schema_dlg: Option<ConfirmDialog>,
    conn_dlg: Option<ConnectionDialog<'a>>,
    db_dlg: Option<DatabaseDialog<'a>>,
    schema_dlg: Option<SchemaDialog<'a>>,
    cmd_bar: Rc<RefCell<CommandBarComponent>>,
    mysql_conns: Rc<RefCell<MySQLConnections>>,
    pg_conns: Rc<RefCell<PGConnections>>,
    mysql_pools: Rc<RefCell<MySQLPools>>,
    pg_pools: Rc<RefCell<PGPools>>,
    config: Rc<RefCell<Config>>,
}

impl<'a> ConnectionListComponent<'a> {
    pub fn new(
        cmd_bar: Rc<RefCell<CommandBarComponent>>,
        mysql_conns: Rc<RefCell<MySQLConnections>>,
        pg_conns: Rc<RefCell<PGConnections>>,
        mysql_pools: Rc<RefCell<MySQLPools>>,
        pg_pools: Rc<RefCell<PGPools>>,
        config: Rc<RefCell<Config>>,
    ) -> Self {
        let tree_items: Vec<TreeItem> = get_all_connections(mysql_conns.clone(), pg_conns.clone())
            .iter()
            .map(|c| {
                TreeItem::Connection(ConnectionItem {
                    id: *c.get_id(),
                    kind: c.get_kind().clone(),
                    name: c.get_name().to_owned(),
                    is_collapsed: true,
                    is_open: false,
                })
            })
            .collect();

        ConnectionListComponent {
            state: ListState::default(),
            tree_items: tree_items.clone(),
            show_items: tree_items,
            cmd_bar,
            new_select: None,
            delete_conn_dlg: None,
            delete_db_dlg: None,
            delete_schema_dlg: None,
            conn_dlg: None,
            db_dlg: None,
            schema_dlg: None,
            mysql_conns,
            pg_conns,
            mysql_pools,
            pg_pools,
            config,
        }
    }
    fn add_database_item(&mut self, db: &dyn DB) {
        if let Some(show_index) = self.state.selected() {
            if let TreeItem::Connection(conn) = &self.show_items[show_index] {
                let tree_index = self
                    .tree_items
                    .iter()
                    .position(|item| {
                        if let TreeItem::Connection(c) = item {
                            c.id == conn.id
                        } else {
                            false
                        }
                    })
                    .unwrap();
                let new_items = [TreeItem::Database(DatabaseItem {
                    id: Uuid::new_v4(),
                    name: db.name().to_string(),
                    kind: db.kind(),
                    is_collapsed: true,
                    conn_id: conn.id,
                    is_conn_collapsed: conn.is_collapsed,
                    is_open: false,
                })];
                self.tree_items
                    .splice(tree_index + 1..tree_index + 1, new_items.clone());
                self.show_items
                    .splice(show_index + 1..show_index + 1, new_items);
            }
        }
    }
    fn add_schema_item(&mut self, schema: &Schema) {
        if let Some(show_index) = self.state.selected() {
            if let TreeItem::Database(db) = &self.show_items[show_index] {
                let tree_index = self
                    .tree_items
                    .iter()
                    .position(|item| {
                        if let TreeItem::Database(d) = item {
                            d.id == db.id
                        } else {
                            false
                        }
                    })
                    .unwrap();
                let schema_id = Uuid::new_v4();
                let new_schema_item = TreeItem::Schema(SchemaItem {
                    id: schema_id,
                    conn_id: db.conn_id,
                    db_id: db.id,
                    db_name: db.name.to_string(),
                    name: schema.name().to_string(),
                    is_collapsed: true,
                    is_db_collapsed: db.is_collapsed,
                });

                let new_items = [
                    new_schema_item.clone(),
                    TreeItem::Query(DatabaseSubItem {
                        conn_id: db.conn_id,
                        db_id: db.id,
                        db_name: db.name.to_string(),
                        kind: DatabaseKind::PostgreSQL,
                        schema_id: Some(schema_id),
                        schema_name: Some(schema.name().to_string()),
                        is_parent_collapsed: true,
                    }),
                    TreeItem::Table(DatabaseSubItem {
                        conn_id: db.conn_id,
                        db_id: db.id,
                        db_name: db.name.to_string(),
                        kind: DatabaseKind::PostgreSQL,
                        schema_id: Some(schema_id),
                        schema_name: Some(schema.name().to_string()),
                        is_parent_collapsed: true,
                    }),
                    TreeItem::View(DatabaseSubItem {
                        conn_id: db.conn_id,
                        db_id: db.id,
                        db_name: db.name.to_string(),
                        kind: DatabaseKind::PostgreSQL,
                        schema_id: Some(schema_id),
                        schema_name: Some(schema.name().to_string()),
                        is_parent_collapsed: true,
                    }),
                ];
                self.tree_items
                    .splice(tree_index + 1..tree_index + 1, new_items);
                self.show_items
                    .splice(show_index + 1..show_index + 1, [new_schema_item]);
            }
        }
    }
    fn save_connection_item(&mut self, conn: &dyn Connect) {
        let conn_item = self.tree_items.iter_mut().find(|item| {
            if let TreeItem::Connection(c) = item {
                c.id == *conn.get_id()
            } else {
                false
            }
        });
        if let Some(item) = conn_item {
            if let TreeItem::Connection(item) = item {
                if item.name.as_str() != conn.get_name() {
                    item.name = conn.get_name().to_string();
                    let show_item = self.show_items.iter_mut().find(|item| {
                        if let TreeItem::Connection(c) = item {
                            c.id == *conn.get_id()
                        } else {
                            false
                        }
                    });
                    if let Some(TreeItem::Connection(item)) = show_item {
                        item.name = conn.get_name().to_string();
                    }
                }
            }
        } else {
            self.tree_items.push(TreeItem::Connection(ConnectionItem {
                id: *conn.get_id(),
                kind: conn.get_kind().clone(),
                name: conn.get_name().to_owned(),
                is_collapsed: true,
                is_open: false,
            }));
            self.show_items.push(TreeItem::Connection(ConnectionItem {
                id: *conn.get_id(),
                kind: conn.get_kind().clone(),
                name: conn.get_name().to_owned(),
                is_collapsed: true,
                is_open: false,
            }));
        }
    }
    fn rename_pg_db_item(&mut self, db_id: &Uuid, name: &str) {
        self.tree_items.iter_mut().for_each(|item| match item {
            TreeItem::Database(db) => {
                if db.id == *db_id {
                    db.name = name.to_string();
                }
            }
            TreeItem::Schema(schema) => {
                if schema.db_id == *db_id {
                    schema.db_name = name.to_string();
                }
            }
            TreeItem::Query(query) => {
                if query.db_id == *db_id {
                    query.db_name = name.to_string();
                }
            }
            TreeItem::Table(table) => {
                if table.db_id == *db_id {
                    table.db_name = name.to_string();
                }
            }
            TreeItem::View(view) => {
                if view.db_id == *db_id {
                    view.db_name = name.to_string();
                }
            }
            _ => (),
        });
        self.show_items.iter_mut().for_each(|item| match item {
            TreeItem::Database(db) => {
                if db.id == *db_id {
                    db.name = name.to_string();
                }
            }
            TreeItem::Schema(schema) => {
                if schema.db_id == *db_id {
                    schema.db_name = name.to_string();
                }
            }
            TreeItem::Query(query) => {
                if query.db_id == *db_id {
                    query.db_name = name.to_string();
                }
            }
            TreeItem::Table(table) => {
                if table.db_id == *db_id {
                    table.db_name = name.to_string();
                }
            }
            TreeItem::View(view) => {
                if view.db_id == *db_id {
                    view.db_name = name.to_string();
                }
            }

            _ => (),
        });
    }
    fn rename_pg_schema_item(&mut self, schema_id: &Uuid, name: &str) {
        self.tree_items.iter_mut().for_each(|item| match item {
            TreeItem::Schema(schema) => {
                if schema.id == *schema_id {
                    schema.name = name.to_string();
                }
            }
            TreeItem::Query(query) => {
                if query.schema_id == Some(*schema_id) {
                    query.schema_name = Some(name.to_string());
                }
            }
            TreeItem::Table(table) => {
                if table.schema_id == Some(*schema_id) {
                    table.schema_name = Some(name.to_string());
                }
            }
            TreeItem::View(view) => {
                if view.schema_id == Some(*schema_id) {
                    view.schema_name = Some(name.to_string());
                }
            }
            _ => (),
        });
        self.show_items.iter_mut().for_each(|item| match item {
            TreeItem::Schema(schema) => {
                if schema.id == *schema_id {
                    schema.name = name.to_string();
                }
            }
            TreeItem::Query(query) => {
                if query.schema_id == Some(*schema_id) {
                    query.schema_name = Some(name.to_string());
                }
            }
            TreeItem::Table(table) => {
                if table.schema_id == Some(*schema_id) {
                    table.schema_name = Some(name.to_string());
                }
            }
            TreeItem::View(view) => {
                if view.schema_id == Some(*schema_id) {
                    view.schema_name = Some(name.to_string());
                }
            }
            _ => (),
        });
    }
    pub fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect, is_active: bool)
    where
        B: Backend,
    {
        let block = Block::default()
            .title("Connections")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if is_active {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            });
        f.render_widget(block, r);

        let items: Vec<ListItem> = self
            .show_items
            .iter()
            .map(|i| match i {
                TreeItem::Connection(conn) => ListItem::new(format!(
                    "{}  {}",
                    if conn.is_collapsed {
                        '\u{25b8}'
                    } else {
                        '\u{25be}'
                    },
                    conn.name,
                )),
                TreeItem::Database(db) => ListItem::new(format!(
                    "  {}  {}",
                    if db.is_collapsed {
                        '\u{25b8}'
                    } else {
                        '\u{25be}'
                    },
                    db.name
                )),
                TreeItem::Schema(schema) => ListItem::new(format!(
                    "    {}  {}",
                    if schema.is_collapsed {
                        '\u{25b8}'
                    } else {
                        '\u{25be}'
                    },
                    schema.name
                )),
                TreeItem::Query(query) => Self::generate_sub_list_item(query, "Query"),
                TreeItem::Table(table) => Self::generate_sub_list_item(table, "Table"),
                TreeItem::View(view) => Self::generate_sub_list_item(view, "View"),
            })
            .collect();

        f.render_stateful_widget(
            List::new(items).highlight_style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            r.inner(&Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.state,
        );
        if is_active {
            self.update_commands();
        }
    }
    pub fn draw_dialog<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        if let Some(select) = self.new_select.as_mut() {
            select.draw(f);
        }
        if let Some(dlg) = self.delete_conn_dlg.as_ref() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.delete_db_dlg.as_ref() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.delete_schema_dlg.as_ref() {
            dlg.draw(f);
        }
        if let Some(conn_dlg) = self.conn_dlg.as_mut() {
            conn_dlg.draw(f);
        }
        if let Some(db_dlg) = self.db_dlg.as_mut() {
            db_dlg.draw(f);
        }
        if let Some(schema_dlg) = self.schema_dlg.as_mut() {
            schema_dlg.draw(f);
        }
    }
    pub async fn handle_event(&mut self, key: &Key) -> Result<ComponentResult> {
        let result = if self.delete_conn_dlg.is_some() {
            self.handle_delete_conn_dlg_event(key).await?
        } else if self.delete_db_dlg.is_some() {
            self.handle_delete_db_dlg_event(key).await?
        } else if self.delete_schema_dlg.is_some() {
            self.handle_delete_schema_dlg_event(key).await?
        } else if self.new_select.is_some() {
            self.handle_new_select_event(key)
        } else if self.conn_dlg.is_some() {
            self.handle_conn_dlg_event(key).await?
        } else if self.db_dlg.is_some() {
            self.handle_db_dlg_event(key).await?
        } else if self.schema_dlg.is_some() {
            self.handle_schema_dlg_event(key).await?
        } else {
            self.handle_main_event(key).await?
        };

        Ok(result)
    }
    async fn handle_delete_conn_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.delete_conn_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_conn_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.state.selected() {
                        if let TreeItem::Connection(conn_item) = self.show_items[index].clone() {
                            match conn_item.kind {
                                DatabaseKind::MySQL => {
                                    close_mysql_pool(
                                        self.mysql_conns.clone(),
                                        self.mysql_pools.clone(),
                                        &conn_item.id,
                                        None,
                                    )
                                    .await;
                                    delete_mysql_connection(
                                        self.mysql_conns.clone(),
                                        &conn_item.id,
                                        self.config.clone(),
                                    )?;
                                }
                                DatabaseKind::PostgreSQL => {
                                    close_pg_pool(
                                        self.pg_conns.clone(),
                                        self.pg_pools.clone(),
                                        &conn_item.id,
                                        None,
                                    )
                                    .await;
                                    delete_pg_connection(
                                        self.pg_conns.clone(),
                                        &conn_item.id,
                                        self.config.clone(),
                                    )?;
                                }
                            }
                            self.tree_items.retain(|item| match item {
                                TreeItem::Connection(conn) => conn.id != conn_item.id,
                                TreeItem::Database(db) => db.conn_id != conn_item.id,
                                TreeItem::Schema(schema) => schema.conn_id != conn_item.id,
                                TreeItem::Query(query) => query.conn_id != conn_item.id,
                                TreeItem::Table(table) => table.conn_id != conn_item.id,
                                TreeItem::View(view) => view.conn_id != conn_item.id,
                            });
                            self.show_items.retain(|item| match item {
                                TreeItem::Connection(conn) => conn.id != conn_item.id,
                                TreeItem::Database(db) => db.conn_id != conn_item.id,
                                TreeItem::Schema(schema) => schema.conn_id != conn_item.id,
                                TreeItem::Query(query) => query.conn_id != conn_item.id,
                                TreeItem::Table(table) => table.conn_id != conn_item.id,
                                TreeItem::View(view) => view.conn_id != conn_item.id,
                            });

                            self.state.select(None);
                            self.delete_conn_dlg = None;
                        }
                    }
                }
                _ => (),
            }
        }

        Ok(ComponentResult::Done)
    }
    async fn handle_delete_db_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match self.delete_db_dlg.as_mut().unwrap().handle_event(key) {
            DialogResult::Cancel => {
                self.delete_db_dlg = None;
            }
            DialogResult::Confirm(_) => {
                if let Some(index) = self.state.selected() {
                    if let TreeItem::Database(db_item) = self.show_items[index].clone() {
                        match db_item.kind {
                            DatabaseKind::MySQL => {
                                execute_mysql_query(
                                    self.mysql_conns.clone(),
                                    self.mysql_pools.clone(),
                                    &db_item.conn_id,
                                    None,
                                    format!("DROP DATABASE `{}`", db_item.name).as_str(),
                                )
                                .await?;
                                close_mysql_pool(
                                    self.mysql_conns.clone(),
                                    self.mysql_pools.clone(),
                                    &db_item.conn_id,
                                    Some(&db_item.name),
                                )
                                .await;
                            }
                            DatabaseKind::PostgreSQL => {
                                execute_pg_query(
                                    self.pg_conns.clone(),
                                    self.pg_pools.clone(),
                                    &db_item.conn_id,
                                    None,
                                    format!("DROP DATABASE \"{}\"", db_item.name).as_str(),
                                )
                                .await?;
                                close_pg_pool(
                                    self.pg_conns.clone(),
                                    self.pg_pools.clone(),
                                    &db_item.conn_id,
                                    Some(&db_item.name),
                                )
                                .await;
                            }
                        }
                        self.tree_items.retain(|item| match item {
                            TreeItem::Connection(_) => true,
                            TreeItem::Database(db) => db.id != db_item.id,
                            TreeItem::Schema(schema) => schema.db_id != db_item.id,
                            TreeItem::Query(query) => query.db_id != db_item.id,
                            TreeItem::Table(table) => table.db_id != db_item.id,
                            TreeItem::View(view) => view.db_id != db_item.id,
                        });
                        self.show_items.retain(|item| match item {
                            TreeItem::Connection(_) => true,
                            TreeItem::Database(db) => db.id != db_item.id,
                            TreeItem::Schema(schema) => schema.db_id != db_item.id,
                            TreeItem::Query(query) => query.db_id != db_item.id,
                            TreeItem::Table(table) => table.db_id != db_item.id,
                            TreeItem::View(view) => view.db_id != db_item.id,
                        });

                        self.state.select(None);
                        self.delete_db_dlg = None;
                    }
                }
            }
            _ => {}
        }

        Ok(ComponentResult::Done)
    }
    async fn handle_delete_schema_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.delete_schema_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_schema_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.state.selected() {
                        if let TreeItem::Schema(schema_item) = self.show_items[index].clone() {
                            execute_pg_query(
                                self.pg_conns.clone(),
                                self.pg_pools.clone(),
                                &schema_item.conn_id,
                                Some(schema_item.db_name.as_str()),
                                format!("DROP SCHEMA \"{}\"", schema_item.name).as_str(),
                            )
                            .await?;
                            self.tree_items.retain(|item| match item {
                                TreeItem::Connection(_) => true,
                                TreeItem::Database(_) => true,
                                TreeItem::Schema(schema) => schema.id != schema_item.id,
                                TreeItem::Query(query) => query.schema_id != Some(schema_item.id),
                                TreeItem::Table(table) => table.schema_id != Some(schema_item.id),
                                TreeItem::View(view) => view.schema_id != Some(schema_item.id),
                            });
                            self.show_items.retain(|item| match item {
                                TreeItem::Connection(_) => true,
                                TreeItem::Database(_) => true,
                                TreeItem::Schema(schema) => schema.id != schema_item.id,
                                TreeItem::Query(query) => query.schema_id != Some(schema_item.id),
                                TreeItem::Table(table) => table.schema_id != Some(schema_item.id),
                                TreeItem::View(view) => view.schema_id != Some(schema_item.id),
                            });
                            self.state.select(None);
                            self.delete_schema_dlg = None;
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(ComponentResult::Done)
    }
    fn handle_new_select_event(&mut self, key: &Key) -> ComponentResult {
        match self.new_select.as_mut().unwrap().handle_event(key) {
            DialogResult::Cancel => {
                self.new_select = None;
            }
            DialogResult::Confirm(kind) => {
                let mut dlg = ConnectionDialog::default();
                match kind {
                    "MySQL" => dlg.set_mysql_connection(None),
                    "PostgreSQL" => dlg.set_pg_connection(None),
                    _ => (),
                }
                self.conn_dlg = Some(dlg);
                self.new_select = None;
            }
            _ => (),
        }
        ComponentResult::Done
    }
    async fn handle_conn_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.conn_dlg.as_mut() {
            match dlg.handle_event(key)? {
                DialogResult::Cancel => {
                    self.conn_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    self.save_connection(&map).await?;
                    self.conn_dlg = None;
                }
                _ => (),
            }
        }

        Ok(ComponentResult::Done)
    }
    async fn handle_db_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.db_dlg.as_mut() {
            match dlg.handle_event(key).await? {
                DialogResult::Cancel => {
                    self.db_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    match dlg.get_mode() {
                        DatabaseMode::Create => self.create_database(&map).await?,
                        DatabaseMode::Edit => self.edit_database(&map).await?,
                    }
                    self.db_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_schema_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.schema_dlg.as_mut() {
            match dlg.handle_event(key).await? {
                DialogResult::Cancel => {
                    self.schema_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    match dlg.get_mode() {
                        SchemaMode::Create => self.create_schema(&map).await?,
                        SchemaMode::Edit => self.edit_schema(&map).await?,
                    }
                    self.schema_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_main_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            USER_KEY => {
                if let Some(index) = self.state.selected() {
                    if let TreeItem::Connection(c) = &self.show_items[index] {
                        match c.kind {
                            DatabaseKind::MySQL => {
                                return Ok(ComponentResult::Goto(Goto::UserListMySQL {
                                    conn_id: c.id,
                                }));
                            }
                            DatabaseKind::PostgreSQL => {
                                return Ok(ComponentResult::Goto(Goto::RoleListPG {
                                    conn_id: c.id,
                                }));
                            }
                        }
                    }
                }
            }
            NEW_KEY => {
                if let Some(index) = self.state.selected() {
                    match &self.show_items[index] {
                        TreeItem::Connection(conn) => {
                            let mut dlg = DatabaseDialog::default();
                            match conn.kind {
                                DatabaseKind::MySQL => {
                                    dlg.set_mysql_db(
                                        self.mysql_conns.clone(),
                                        self.mysql_pools.clone(),
                                        &conn.id,
                                        None,
                                    )
                                    .await?;
                                }
                                DatabaseKind::PostgreSQL => {
                                    dlg.set_pg_db(
                                        self.pg_conns.clone(),
                                        self.pg_pools.clone(),
                                        &conn.id,
                                        None,
                                    )
                                    .await?;
                                }
                            }
                            self.db_dlg = Some(dlg);
                        }
                        TreeItem::Database(db) => {
                            if let DatabaseKind::PostgreSQL = db.kind {
                                let pool = get_pg_pool(
                                    self.pg_conns.clone(),
                                    self.pg_pools.clone(),
                                    &db.conn_id,
                                    Some(&db.name),
                                )
                                .await?;
                                let roles = get_pg_role_names(&pool).await?;
                                self.schema_dlg = Some(SchemaDialog::new(roles, None));
                            }
                        }
                        _ => (),
                    }
                } else {
                    self.new_select = Some(Select::new(
                        "Database Type".to_string(),
                        DatabaseKind::iter().map(|db| db.to_string()).collect(),
                        None,
                    ));
                }
            }
            EDIT_KEY => {
                self.handle_edit_event().await?;
            }
            DELETE_KEY => {
                self.handle_delete_event();
            }
            UP_KEY => {
                if !self.show_items.is_empty() {
                    let index = get_table_up_index(self.state.selected());
                    self.state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.show_items.is_empty() {
                    let index = get_table_down_index(self.state.selected(), self.show_items.len());
                    self.state.select(Some(index));
                }
            }
            CANCEL_KEY => {
                self.state.select(None);
            }
            RIGHT_KEY => {
                return Ok(ComponentResult::Focus(Focus::MainPanel));
            }
            CONFIRM_KEY => {
                if let Some(index) = self.state.selected() {
                    match self.show_items[index].clone() {
                        TreeItem::Connection(conn_item) => {
                            self.set_conn_items_collapsed(&conn_item, !conn_item.is_collapsed)
                                .await?;
                        }
                        TreeItem::Database(db_item) => {
                            self.set_db_items_collapsed(&db_item, !db_item.is_collapsed)
                                .await?;
                        }
                        TreeItem::Schema(schema_item) => {
                            self.set_schema_items_collapsed(
                                &schema_item,
                                !schema_item.is_collapsed,
                            );
                        }
                        TreeItem::Query(query) => {
                            return Ok(ComponentResult::Goto(Goto::QueryList {
                                conn_id: query.conn_id,
                                db_name: query.db_name.clone(),
                                kind: query.kind,
                            }));
                        }
                        TreeItem::Table(table) => match table.kind {
                            DatabaseKind::MySQL => {
                                return Ok(ComponentResult::Goto(Goto::TableListMySQL {
                                    conn_id: table.conn_id,
                                    db_name: table.db_name,
                                }));
                            }
                            DatabaseKind::PostgreSQL => {
                                return Ok(ComponentResult::Goto(Goto::TableListPG {
                                    conn_id: table.conn_id,
                                    db_name: table.db_name.clone(),
                                    schema_name: table.schema_name.unwrap(),
                                }));
                            }
                        },
                        TreeItem::View(view) => match view.kind {
                            DatabaseKind::MySQL => {
                                return Ok(ComponentResult::Goto(Goto::ViewListMySQL {
                                    conn_id: view.conn_id,
                                    db_name: view.db_name,
                                }));
                            }
                            DatabaseKind::PostgreSQL => {
                                return Ok(ComponentResult::Goto(Goto::ViewListPG {
                                    conn_id: view.conn_id,
                                    db_name: view.db_name.clone(),
                                    schema_name: view.schema_name.unwrap(),
                                }));
                            }
                        },
                    }
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_edit_event(&mut self) -> Result<()> {
        if let Some(index) = self.state.selected() {
            match &self.show_items[index] {
                TreeItem::Connection(conn) => {
                    let mut dlg = ConnectionDialog::default();
                    match conn.kind {
                        DatabaseKind::MySQL => {
                            let c = get_mysql_connection(self.mysql_conns.clone(), &conn.id)?;
                            dlg.set_mysql_connection(Some(&c));
                        }
                        DatabaseKind::PostgreSQL => {
                            let c = get_pg_connection(self.pg_conns.clone(), &conn.id)?;
                            dlg.set_pg_connection(Some(&c));
                        }
                    }
                    self.conn_dlg = Some(dlg);
                }
                TreeItem::Database(db_item) => {
                    let mut dlg = DatabaseDialog::default();
                    match db_item.kind {
                        DatabaseKind::MySQL => {
                            let db = get_mysql_database(
                                self.mysql_conns.clone(),
                                self.mysql_pools.clone(),
                                &db_item.conn_id,
                                &db_item.name,
                            )
                            .await?;
                            dlg.set_mysql_db(
                                self.mysql_conns.clone(),
                                self.mysql_pools.clone(),
                                &db_item.conn_id,
                                Some(&db),
                            )
                            .await?;
                        }
                        DatabaseKind::PostgreSQL => {
                            let db = get_pg_database(
                                self.pg_conns.clone(),
                                self.pg_pools.clone(),
                                &db_item.conn_id,
                                &db_item.name,
                            )
                            .await?;
                            dlg.set_pg_db(
                                self.pg_conns.clone(),
                                self.pg_pools.clone(),
                                &db_item.conn_id,
                                Some(&db),
                            )
                            .await?;
                        }
                    }
                    self.db_dlg = Some(dlg);
                }
                TreeItem::Schema(schema_item) => {
                    let schema = get_pg_schema(
                        self.pg_conns.clone(),
                        self.pg_pools.clone(),
                        &schema_item.conn_id,
                        Some(schema_item.db_name.as_str()),
                        &schema_item.name,
                    )
                    .await?;
                    let pool = get_pg_pool(
                        self.pg_conns.clone(),
                        self.pg_pools.clone(),
                        &schema_item.conn_id,
                        Some(&schema_item.db_name),
                    )
                    .await?;
                    let roles = get_pg_role_names(&pool).await?;

                    self.schema_dlg = Some(SchemaDialog::new(roles, Some(&schema)));
                }
                _ => (),
            }
        }
        Ok(())
    }
    fn handle_delete_event(&mut self) {
        let current_index = self.state.selected();
        if let Some(index) = current_index {
            let selected_item = self.show_items.get(index).unwrap().clone();
            match selected_item {
                TreeItem::Connection(_) => {
                    self.delete_conn_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Warning,
                        "Delete Connection",
                        "Are you sure to delete this connection?",
                    ));
                }
                TreeItem::Database(_) => {
                    self.delete_db_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Warning,
                        "Delete Database",
                        "Are you sure to delete this database?",
                    ));
                }
                TreeItem::Schema(_) => {
                    self.delete_schema_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Warning,
                        "Delete Schema",
                        "Are you sure to delete this schema?",
                    ));
                }
                _ => (),
            }
        }
    }
    async fn save_connection(&mut self, map: &HashMap<String, Option<String>>) -> Result<()> {
        let kind = DatabaseKind::try_from(map.get("kind").unwrap().as_deref().unwrap())?;
        match kind {
            DatabaseKind::MySQL => {
                let conn = self.generate_mysql_connection(map)?;
                close_mysql_pool(
                    self.mysql_conns.clone(),
                    self.mysql_pools.clone(),
                    &conn.id,
                    None,
                )
                .await;
                save_mysql_connection(self.mysql_conns.clone(), self.config.clone(), &conn).await?;
                self.save_connection_item(&conn);
            }
            DatabaseKind::PostgreSQL => {
                let conn = self.generate_pg_connection(map)?;
                close_pg_pool(self.pg_conns.clone(), self.pg_pools.clone(), &conn.id, None).await;
                save_pg_connection(self.pg_conns.clone(), self.config.clone(), &conn).await?;
                self.save_connection_item(&conn);
            }
        }
        Ok(())
    }
    async fn create_database(&mut self, map: &HashMap<String, Option<String>>) -> Result<()> {
        let kind = DatabaseKind::try_from(map.get("kind").unwrap().as_deref().unwrap())?;
        let conn_id = Uuid::parse_str(map.get("conn_id").unwrap().as_ref().unwrap())?;
        match kind {
            DatabaseKind::MySQL => {
                let db = self.generate_mysql_database(map)?;
                execute_mysql_query(
                    self.mysql_conns.clone(),
                    self.mysql_pools.clone(),
                    &conn_id,
                    None,
                    &db.get_create_ddl(),
                )
                .await?;
                self.add_database_item(&db);
            }
            DatabaseKind::PostgreSQL => {
                let db = self.generate_pg_database(map)?;
                execute_pg_query(
                    self.pg_conns.clone(),
                    self.pg_pools.clone(),
                    &conn_id,
                    None,
                    &db.get_create_ddl(),
                )
                .await?;
                self.add_database_item(&db);
            }
        }
        Ok(())
    }

    async fn edit_database(&mut self, map: &HashMap<String, Option<String>>) -> Result<()> {
        let kind = DatabaseKind::try_from(map.get("kind").unwrap().as_deref().unwrap())?;
        let conn_id = Uuid::parse_str(map.get("conn_id").unwrap().as_deref().unwrap())?;

        if let Some(index) = self.state.selected() {
            if let TreeItem::Database(db_item) = self.show_items[index].clone() {
                match kind {
                    DatabaseKind::MySQL => {
                        let db = self.generate_mysql_database(map)?;
                        let old_db = get_mysql_database(
                            self.mysql_conns.clone(),
                            self.mysql_pools.clone(),
                            &db_item.conn_id,
                            &db_item.name,
                        )
                        .await?;
                        let sql = db.get_alter_ddl(&old_db);

                        if !sql.is_empty() {
                            close_mysql_pool(
                                self.mysql_conns.clone(),
                                self.mysql_pools.clone(),
                                &db_item.conn_id,
                                Some(&db_item.name),
                            )
                            .await;
                            execute_mysql_query_unprepared(
                                self.mysql_conns.clone(),
                                self.mysql_pools.clone(),
                                &conn_id,
                                None,
                                &db.get_alter_ddl(&old_db),
                            )
                            .await?;
                        }
                    }
                    DatabaseKind::PostgreSQL => {
                        let db = self.generate_pg_database(map)?;
                        let old_db = get_pg_database(
                            self.pg_conns.clone(),
                            self.pg_pools.clone(),
                            &conn_id,
                            &db_item.name,
                        )
                        .await?;

                        if old_db.name() != db.name() {
                            execute_pg_query_unprepared(
                                self.pg_conns.clone(),
                                self.pg_pools.clone(),
                                &conn_id,
                                &db.get_rename_ddl(old_db.name()),
                            )
                            .await?;
                        }
                        if old_db.get_owner() != db.get_owner() {
                            execute_pg_query(
                                self.pg_conns.clone(),
                                self.pg_pools.clone(),
                                &conn_id,
                                None,
                                &db.get_alter_owner_ddl(),
                            )
                            .await?;
                        }
                        if old_db.get_table_space() != db.get_table_space() {
                            if let Some(ddl) = &db.get_alter_tablespace_ddl() {
                                execute_pg_query(
                                    self.pg_conns.clone(),
                                    self.pg_pools.clone(),
                                    &conn_id,
                                    None,
                                    ddl,
                                )
                                .await?;
                            }
                        }
                        if old_db.get_allow_conn() != db.get_allow_conn()
                            || old_db.get_conn_limit() != db.get_conn_limit()
                            || old_db.get_is_template() != db.get_is_template()
                        {
                            execute_pg_query(
                                self.pg_conns.clone(),
                                self.pg_pools.clone(),
                                &conn_id,
                                None,
                                &db.get_alter_options_ddl(
                                    old_db.get_allow_conn(),
                                    old_db.get_conn_limit(),
                                    old_db.get_is_template(),
                                ),
                            )
                            .await?;
                        }
                        if old_db.name() != db.name() {
                            self.rename_pg_db_item(&db_item.id, db.name());
                        }
                    }
                }
            }
        }
        Ok(())
    }
    async fn create_schema(&mut self, map: &HashMap<String, Option<String>>) -> Result<()> {
        if let Some(index) = self.state.selected() {
            if let TreeItem::Database(db_item) = &self.show_items[index] {
                let schema = self.generate_pg_schema(map)?;
                execute_pg_query(
                    self.pg_conns.clone(),
                    self.pg_pools.clone(),
                    &db_item.conn_id,
                    Some(db_item.name.as_str()),
                    &schema.get_create_ddl(),
                )
                .await?;
                self.add_schema_item(&schema);
            }
        }

        Ok(())
    }
    async fn edit_schema(&mut self, map: &HashMap<String, Option<String>>) -> Result<()> {
        if let Some(index) = self.state.selected() {
            if let TreeItem::Schema(schema_item) = self.show_items[index].clone() {
                let schema = self.generate_pg_schema(map)?;
                let old_schema = get_pg_schema(
                    self.pg_conns.clone(),
                    self.pg_pools.clone(),
                    &schema_item.conn_id,
                    Some(schema_item.db_name.as_str()),
                    &schema_item.name,
                )
                .await?;

                if old_schema.name() != schema.name() {
                    execute_pg_query(
                        self.pg_conns.clone(),
                        self.pg_pools.clone(),
                        &schema_item.conn_id,
                        Some(schema_item.db_name.as_str()),
                        &schema.get_rename_ddl(old_schema.name()),
                    )
                    .await?;
                }
                if old_schema.owner() != schema.owner() {
                    execute_pg_query(
                        self.pg_conns.clone(),
                        self.pg_pools.clone(),
                        &schema_item.conn_id,
                        Some(schema_item.db_name.as_str()),
                        &schema.get_alter_owner_ddl(),
                    )
                    .await?;
                }
                if old_schema.name() != schema.name() {
                    self.rename_pg_schema_item(&schema_item.id, schema.name());
                }
            }
        }
        Ok(())
    }
    fn generate_mysql_connection(
        &self,
        map: &HashMap<String, Option<String>>,
    ) -> Result<MySQLConnection> {
        Ok(MySQLConnection {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id)?
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map
                .get("name")
                .unwrap()
                .as_ref()
                .ok_or_else(|| Error::msg("cannot get name"))?
                .to_string(),
            host: map
                .get("host")
                .unwrap()
                .as_ref()
                .ok_or_else(|| Error::msg("cannot get host"))?
                .to_string(),
            port: map
                .get("port")
                .unwrap()
                .as_ref()
                .unwrap_or(&String::from("3306"))
                .to_string(),
            user: map
                .get("user")
                .unwrap()
                .as_ref()
                .ok_or_else(|| Error::msg("cannot get user"))?
                .to_string(),
            password: map
                .get("password")
                .unwrap()
                .as_ref()
                .ok_or_else(|| Error::msg("cannot get password"))?
                .to_string(),
            add_at: Utc::now(),
        })
    }
    fn generate_pg_connection(
        &self,
        map: &HashMap<String, Option<String>>,
    ) -> Result<PGConnection> {
        Ok(PGConnection {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id)?
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map
                .get("name")
                .unwrap()
                .as_ref()
                .ok_or_else(|| Error::msg("cannot get name"))?
                .to_string(),
            host: map
                .get("host")
                .unwrap()
                .as_ref()
                .ok_or_else(|| Error::msg("cannot get host"))?
                .to_string(),
            port: map
                .get("port")
                .unwrap()
                .as_ref()
                .map(|p| p.to_string())
                .unwrap_or(String::from("3306")),
            init_db: map.get("init db").unwrap().as_ref().map(|s| s.to_string()),
            user: map
                .get("user")
                .unwrap()
                .as_ref()
                .ok_or_else(|| Error::msg("cannot get user"))?
                .to_string(),
            password: map
                .get("password")
                .unwrap()
                .as_ref()
                .ok_or_else(|| Error::msg("cannot get passowrd"))?
                .to_string(),
            add_at: Utc::now(),
        })
    }
    fn generate_mysql_database(
        &self,
        map: &HashMap<String, Option<String>>,
    ) -> Result<MySQLDatabase> {
        Ok(MySQLDatabase {
            name: map
                .get("name")
                .unwrap()
                .as_ref()
                .ok_or_else(|| Error::msg("cannot get name"))?
                .to_string(),
            character_set: map
                .get("character set")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            collation: map
                .get("collation")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
        })
    }

    fn generate_pg_database(&self, map: &HashMap<String, Option<String>>) -> Result<PGDatabase> {
        Ok(PGDatabase {
            name: map
                .get("name")
                .unwrap()
                .as_ref()
                .ok_or_else(|| Error::msg("cannot get name"))?
                .to_string(),
            owner: map.get("owner").unwrap().as_ref().map(|s| s.to_string()),
            collation_order: map
                .get("collation order")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            character_class: map
                .get("character class")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            template: map.get("template").unwrap().as_ref().map(|s| s.to_string()),
            tablespace: map
                .get("tablespace")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            connection_limit: map
                .get("connection limit")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            allow_connection: matches!(
                map.get("allow connection")
                    .unwrap()
                    .as_ref()
                    .ok_or_else(|| Error::msg("cannot get allow connection"))?
                    .as_str(),
                "true" | "True"
            ),
            is_template: matches!(
                map.get("is template")
                    .unwrap()
                    .as_ref()
                    .ok_or_else(|| Error::msg("canot get is template"))?
                    .as_str(),
                "true" | "True"
            ),
        })
    }
    fn generate_pg_schema(&self, map: &HashMap<String, Option<String>>) -> Result<Schema> {
        Ok(Schema {
            name: map
                .get("name")
                .unwrap()
                .as_ref()
                .ok_or_else(|| Error::msg("cannot get name"))?
                .to_string(),
            owner: map.get("owner").unwrap().as_ref().map(|s| s.to_string()),
        })
    }
    async fn set_conn_items_collapsed(
        &mut self,
        conn_item: &ConnectionItem,
        is_collapsed: bool,
    ) -> Result<()> {
        self.tree_items.iter_mut().for_each(|item| match item {
            TreeItem::Connection(conn) => {
                if conn.id == conn_item.id {
                    conn.is_collapsed = is_collapsed;
                }
            }
            TreeItem::Database(db) => {
                if db.conn_id == conn_item.id {
                    db.is_conn_collapsed = is_collapsed;
                    if is_collapsed {
                        db.is_collapsed = true;
                    }
                }
            }
            TreeItem::Schema(schema) => {
                if schema.conn_id == conn_item.id && is_collapsed {
                    schema.is_db_collapsed = true;
                    schema.is_collapsed = true;
                }
            }
            TreeItem::Query(query) => {
                if query.conn_id == conn_item.id && is_collapsed {
                    query.is_parent_collapsed = true;
                }
            }
            TreeItem::Table(table) => {
                if table.conn_id == conn_item.id && is_collapsed {
                    table.is_parent_collapsed = true;
                }
            }
            TreeItem::View(view) => {
                if view.conn_id == conn_item.id && is_collapsed {
                    view.is_parent_collapsed = true;
                }
            }
        });
        if !is_collapsed && !conn_item.is_open {
            let tree_items = match conn_item.kind {
                DatabaseKind::MySQL => {
                    let pool = get_mysql_pool(
                        self.mysql_conns.clone(),
                        self.mysql_pools.clone(),
                        &conn_item.id,
                        None,
                    )
                    .await?;

                    let databases = get_mysql_databases(&pool).await?;

                    Self::create_mysql_database_items(conn_item, &databases)
                }
                DatabaseKind::PostgreSQL => {
                    let pool = get_pg_pool(
                        self.pg_conns.clone(),
                        self.pg_pools.clone(),
                        &conn_item.id,
                        None,
                    )
                    .await?;
                    let databases = get_pg_databases(&pool).await?;

                    Self::create_pg_database_items(conn_item, &databases)
                }
            };
            let index = self
                .tree_items
                .iter()
                .position(|item| {
                    if let TreeItem::Connection(item) = item {
                        item.id == conn_item.id
                    } else {
                        false
                    }
                })
                .unwrap();
            self.tree_items.splice(index + 1..index + 1, tree_items);
            self.tree_items.iter_mut().for_each(|item| {
                if let TreeItem::Connection(c) = item {
                    if c.id == conn_item.id {
                        c.is_open = true;
                    }
                }
            });
        }
        self.show_items = self
            .tree_items
            .iter()
            .cloned()
            .filter(|i| match i {
                TreeItem::Connection(_) => true,
                TreeItem::Database(db) => !db.is_conn_collapsed,
                TreeItem::Schema(schema) => !schema.is_db_collapsed,
                TreeItem::Query(query) => !query.is_parent_collapsed,
                TreeItem::Table(table) => !table.is_parent_collapsed,
                TreeItem::View(view) => !view.is_parent_collapsed,
            })
            .collect();

        Ok(())
    }
    async fn set_db_items_collapsed(
        &mut self,
        db_item: &DatabaseItem,
        is_collapsed: bool,
    ) -> Result<()> {
        self.tree_items.iter_mut().for_each(|item| match item {
            TreeItem::Connection(_) => {}
            TreeItem::Database(database) => {
                if database.id == db_item.id {
                    database.is_collapsed = is_collapsed;
                }
            }
            TreeItem::Schema(schema) => {
                if schema.db_id == db_item.id {
                    schema.is_db_collapsed = is_collapsed;
                    if is_collapsed {
                        schema.is_collapsed = true;
                    }
                }
            }
            TreeItem::Query(query) => {
                if query.db_id == db_item.id {
                    if query.schema_name.is_none() {
                        query.is_parent_collapsed = is_collapsed;
                    } else if is_collapsed {
                        query.is_parent_collapsed = true;
                    }
                }
            }
            TreeItem::Table(table) => {
                if table.db_id == db_item.id {
                    if table.schema_name.is_none() {
                        table.is_parent_collapsed = is_collapsed;
                    } else if is_collapsed {
                        table.is_parent_collapsed = true;
                    }
                }
            }
            TreeItem::View(view) => {
                if view.db_id == db_item.id {
                    if view.schema_name.is_none() {
                        view.is_parent_collapsed = is_collapsed;
                    } else if is_collapsed {
                        view.is_parent_collapsed = true;
                    }
                }
            }
        });

        if !is_collapsed && !db_item.is_open {
            let tree_items = match db_item.kind {
                DatabaseKind::MySQL => Self::create_mysql_database_sub_items(db_item),
                DatabaseKind::PostgreSQL => {
                    let schemas = get_pg_schemas(
                        self.pg_conns.clone(),
                        self.pg_pools.clone(),
                        &db_item.conn_id,
                        Some(db_item.name.as_str()),
                    )
                    .await?;

                    Self::create_pg_database_sub_items(db_item, &schemas)
                }
            };
            let index = self
                .tree_items
                .iter()
                .position(|i| {
                    if let TreeItem::Database(d) = i {
                        d.id == db_item.id
                    } else {
                        false
                    }
                })
                .unwrap();

            self.tree_items.splice(index + 1..index + 1, tree_items);
            self.tree_items.iter_mut().for_each(|item| {
                if let TreeItem::Database(d) = item {
                    if d.id == db_item.id {
                        d.is_open = true;
                    }
                }
            });
        }
        self.show_items = self
            .tree_items
            .iter()
            .cloned()
            .filter(|i| match i {
                TreeItem::Connection(_) => true,
                TreeItem::Database(db) => !db.is_conn_collapsed,
                TreeItem::Schema(schema) => !schema.is_db_collapsed,
                TreeItem::Query(query) => !query.is_parent_collapsed,
                TreeItem::Table(table) => !table.is_parent_collapsed,
                TreeItem::View(view) => !view.is_parent_collapsed,
            })
            .collect();

        Ok(())
    }
    fn set_schema_items_collapsed(&mut self, schema_item: &SchemaItem, is_collapsed: bool) {
        self.tree_items.iter_mut().for_each(|item| match item {
            TreeItem::Connection(_) => (),
            TreeItem::Database(_) => (),
            TreeItem::Schema(s) => {
                if schema_item.id == s.id {
                    s.is_collapsed = is_collapsed;
                }
            }
            TreeItem::Query(query) => {
                if query.schema_id == Some(schema_item.id) {
                    query.is_parent_collapsed = is_collapsed;
                }
            }
            TreeItem::Table(table) => {
                if table.schema_id == Some(schema_item.id) {
                    table.is_parent_collapsed = is_collapsed;
                }
            }
            TreeItem::View(view) => {
                if view.schema_id == Some(schema_item.id) {
                    view.is_parent_collapsed = is_collapsed;
                }
            }
        });
        self.show_items = self
            .tree_items
            .iter()
            .cloned()
            .filter(|i| match i {
                TreeItem::Connection(_) => true,
                TreeItem::Database(db) => !db.is_conn_collapsed,
                TreeItem::Schema(schema) => !schema.is_db_collapsed,
                TreeItem::Query(query) => !query.is_parent_collapsed,
                TreeItem::Table(table) => !table.is_parent_collapsed,
                TreeItem::View(view) => !view.is_parent_collapsed,
            })
            .collect();
    }
    fn update_commands(&self) {
        let mut cmds = if let Some(dlg) = self.conn_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.db_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.schema_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.delete_conn_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.delete_db_dlg.as_ref() {
            dlg.get_commands()

        } else if let Some(select) = self.new_select.as_ref() {
            select.get_commands()
        
        } else {
            self.get_main_commands()
        };
        self.cmd_bar.borrow_mut().set_commands(&mut cmds);
    }
    fn get_main_commands(&self) -> Vec<Command> {
        let mut cmds = vec![];
        if let Some(index) = self.state.selected() {
            let item = &self.show_items[index];
            match item {
                TreeItem::Connection(_) => cmds.append(&mut vec![
                    Command {
                        name: "New Database",
                        key: NEW_KEY,
                    },
                    Command {
                        name: "Edit",
                        key: EDIT_KEY,
                    },
                    Command {
                        name: "Delete",
                        key: DELETE_KEY,
                    },
                    Command {
                        name: "Users",
                        key: USER_KEY,
                    },
                    Command {
                        name: "Open/Close",
                        key: CONFIRM_KEY,
                    },
                ]),
                TreeItem::Database(db) => {
                    if db.kind == DatabaseKind::PostgreSQL {
                        cmds.push(Command {
                            name: "New Schema",
                            key: NEW_KEY,
                        });
                    }
                    cmds.append(&mut vec![
                        Command {
                            name: "Edit",
                            key: EDIT_KEY,
                        },
                        Command {
                            name: "Delete",
                            key: DELETE_KEY,
                        },
                        Command {
                            name: "Open/Close",
                            key: CONFIRM_KEY,
                        },
                    ])
                }
                TreeItem::Schema(_) => cmds.append(&mut vec![
                    Command {
                        name: "Edit",
                        key: EDIT_KEY,
                    },
                    Command {
                        name: "Delete",
                        key: DELETE_KEY,
                    },
                    Command {
                        name: "Open/Close",
                        key: CONFIRM_KEY,
                    },
                ]),
                _ => cmds.append(&mut vec![Command {
                    name: "Open",
                    key: CONFIRM_KEY,
                }]),
            }
            if index != 0 {
                cmds.push(Command {
                    name: "Up",
                    key: UP_KEY,
                });
            }

            if index < self.show_items.len() - 1 {
                cmds.push(Command {
                    name: "Down",
                    key: DOWN_KEY,
                });
            }
            cmds.push(Command {
                name: "Cancel",
                key: CANCEL_KEY,
            });
        } else {
            cmds.push(Command {
                name: "New Connection",
                key: NEW_KEY,
            });
            if !self.show_items.is_empty() {
                cmds.push(Command {
                    name: "Down",
                    key: DOWN_KEY,
                });
            }
        }
        cmds
    }
    fn create_mysql_database_items(
        conn_item: &ConnectionItem,
        databases: &[MySQLDatabase],
    ) -> Vec<TreeItem> {
        databases
            .iter()
            .map(|db| {
                TreeItem::Database(DatabaseItem {
                    id: Uuid::new_v4(),
                    name: db.name().to_owned(),
                    kind: db.kind(),
                    is_collapsed: true,
                    conn_id: conn_item.id,
                    is_conn_collapsed: false,
                    is_open: false,
                })
            })
            .collect()
    }
    fn create_mysql_database_sub_items(db_item: &DatabaseItem) -> Vec<TreeItem> {
        vec![
            TreeItem::Query(DatabaseSubItem {
                conn_id: db_item.conn_id,
                db_id: db_item.id,
                db_name: db_item.name.to_string(),
                kind: DatabaseKind::MySQL,
                schema_id: None,
                schema_name: None,
                is_parent_collapsed: false,
            }),
            TreeItem::Table(DatabaseSubItem {
                conn_id: db_item.conn_id,
                db_id: db_item.id,
                db_name: db_item.name.to_string(),
                kind: DatabaseKind::MySQL,
                schema_id: None,
                schema_name: None,
                is_parent_collapsed: false,
            }),
            TreeItem::View(DatabaseSubItem {
                conn_id: db_item.conn_id,
                db_id: db_item.id,
                db_name: db_item.name.to_string(),
                kind: DatabaseKind::MySQL,
                schema_id: None,
                schema_name: None,
                is_parent_collapsed: false,
            }),
        ]
    }
    fn create_pg_database_items(
        conn_item: &ConnectionItem,
        databases: &[PGDatabase],
    ) -> Vec<TreeItem> {
        databases
            .iter()
            .map(|db| {
                TreeItem::Database(DatabaseItem {
                    id: Uuid::new_v4(),
                    conn_id: conn_item.id,
                    name: db.name().to_owned(),
                    kind: db.kind(),
                    is_collapsed: true,
                    is_conn_collapsed: false,
                    is_open: false,
                })
            })
            .collect::<Vec<TreeItem>>()
    }
    fn create_pg_database_sub_items(db_item: &DatabaseItem, schemas: &[Schema]) -> Vec<TreeItem> {
        schemas
            .iter()
            .flat_map(|schema| {
                let schema_id = Uuid::new_v4();
                vec![
                    TreeItem::Schema(SchemaItem {
                        id: schema_id,
                        db_id: db_item.id,
                        name: schema.name().to_string(),
                        conn_id: db_item.conn_id,
                        db_name: db_item.name.clone(),
                        is_collapsed: true,
                        is_db_collapsed: false,
                    }),
                    TreeItem::Query(DatabaseSubItem {
                        conn_id: db_item.conn_id,
                        db_id: db_item.id,
                        db_name: db_item.name.clone(),
                        kind: DatabaseKind::PostgreSQL,
                        schema_id: Some(schema_id),
                        schema_name: Some(schema.name().to_string()),
                        is_parent_collapsed: true,
                    }),
                    TreeItem::Table(DatabaseSubItem {
                        conn_id: db_item.conn_id,
                        db_id: db_item.id,
                        db_name: db_item.name.clone(),
                        kind: DatabaseKind::PostgreSQL,
                        schema_id: Some(schema_id),
                        schema_name: Some(schema.name().to_string()),
                        is_parent_collapsed: true,
                    }),
                    TreeItem::View(DatabaseSubItem {
                        conn_id: db_item.conn_id,
                        db_id: db_item.id,
                        db_name: db_item.name.clone(),
                        kind: DatabaseKind::PostgreSQL,
                        schema_id: Some(schema_id),
                        schema_name: Some(schema.name().to_string()),
                        is_parent_collapsed: true,
                    }),
                ]
            })
            .collect::<Vec<TreeItem>>()
    }
    fn generate_sub_list_item<'b>(item: &DatabaseSubItem, title: &str) -> ListItem<'b> {
        if item.schema_name.is_some() {
            ListItem::new(format!("      {}  {}", '\u{25b8}', title,))
        } else {
            ListItem::new(format!("    {}  {}", '\u{25b8}', title,))
        }
    }
}
