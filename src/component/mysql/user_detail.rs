use crate::{
    app::{ComponentResult, DialogResult, MainPanel},
    component::{get_table_down_index, get_table_up_index, Command, CommandBarComponent},
    dialog::{confirm::Kind as ConfirmKind, mysql::PrivilegeDialog, ConfirmDialog},
    event::{config::*, Key},
    model::mysql::{
        get_mysql_user, get_mysql_user_member_ofs, get_mysql_user_members,
        get_mysql_user_privileges, get_mysql_users, get_mysql_version, Connections, Privilege,
        User, UserMember, Version,
    },
    pool::{execute_mysql_query_unprepared, fetch_mysql_query, get_mysql_pool, MySQLPools},
    widget::{Form, FormItem},
};
use anyhow::{Error, Result};
use sqlx::Row;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::Spans,
    widgets::{Block, BorderType, Borders, Row as RowUI, Table, TableState, Tabs},
    Frame,
};
use tui_textarea::TextArea;
use uuid::Uuid;

enum PanelKind {
    General,
    Advanced,
    MemberOf,
    Members,
    ServerPrivs,
    Privileges,
    SQLPreview,
}

pub struct UserDetailComponent<'a> {
    conn_id: Option<Uuid>,
    db_version: Version,
    old_user: Option<User>,
    info_dlg: Option<ConfirmDialog>,
    exit_dlg: Option<ConfirmDialog>,
    panel: PanelKind,
    member_ofs: Vec<UserMember>,
    old_member_ofs: Vec<UserMember>,
    members: Vec<UserMember>,
    old_members: Vec<UserMember>,
    member_ofs_state: TableState,
    members_state: TableState,
    srv_privs: HashMap<&'static str, bool>,
    srv_priv_state: TableState,
    privileges: Vec<Privilege>,
    old_privileges: Vec<Privilege>,
    privileges_state: TableState,
    sql_preview: TextArea<'a>,
    form: Form<'a>,
    adv_form: Form<'a>,
    privilege_dlg: Option<PrivilegeDialog<'a>>,
    delete_privilege_dlg: Option<ConfirmDialog>,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<MySQLPools>>,
    cmd_bar: Rc<RefCell<CommandBarComponent>>,
}

impl<'a> UserDetailComponent<'a> {
    pub fn new(
        cmd_bar: Rc<RefCell<CommandBarComponent>>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<MySQLPools>>,
    ) -> Self {
        let mut srv_privs = HashMap::new();
        srv_privs.insert("Alter", false);
        srv_privs.insert("Alter Routine", false);
        srv_privs.insert("Create", false);
        srv_privs.insert("Create Routine", false);
        srv_privs.insert("Create Temporary Tables", false);
        srv_privs.insert("Create User", false);
        srv_privs.insert("Create View", false);
        srv_privs.insert("Delete", false);
        srv_privs.insert("Drop", false);
        srv_privs.insert("Event", false);
        srv_privs.insert("Execute", false);
        srv_privs.insert("File", false);
        srv_privs.insert("Grant Option", false);
        srv_privs.insert("Index", false);
        srv_privs.insert("Insert", false);
        srv_privs.insert("Lock Tables", false);
        srv_privs.insert("Process", false);
        srv_privs.insert("References", false);
        srv_privs.insert("Reload", false);
        srv_privs.insert("Replication Client", false);
        srv_privs.insert("Replication Slave", false);
        srv_privs.insert("Select", false);
        srv_privs.insert("Show Databases", false);
        srv_privs.insert("Show View", false);
        srv_privs.insert("Shutdown", false);
        srv_privs.insert("Super", false);
        srv_privs.insert("Trigger", false);
        srv_privs.insert("Update", false);

        UserDetailComponent {
            conn_id: None,
            db_version: Version::Eight,
            old_user: None,
            info_dlg: None,
            exit_dlg: None,
            panel: PanelKind::General,
            member_ofs: Vec::new(),
            old_member_ofs: Vec::new(),
            member_ofs_state: TableState::default(),
            members: Vec::new(),
            old_members: Vec::new(),
            members_state: TableState::default(),
            privileges: Vec::new(),
            old_privileges: Vec::new(),
            privilege_dlg: None,
            delete_privilege_dlg: None,
            privileges_state: TableState::default(),
            form: Form::default(),
            adv_form: Form::default(),
            srv_privs,
            srv_priv_state: TableState::default(),
            sql_preview: TextArea::default(),
            conns,
            pools,
            cmd_bar,
        }
    }
    pub async fn set_data(
        &mut self,
        conn_id: &Uuid,
        host_name: Option<&str>,
        user_name: Option<&str>,
    ) -> Result<()> {
        self.conn_id = Some(*conn_id);
        self.db_version =
            get_mysql_version(self.conns.clone(), self.pools.clone(), conn_id).await?;
        let pool = get_mysql_pool(
            self.conns.clone(),
            self.pools.clone(),
            self.conn_id.as_ref().unwrap(),
            None,
        )
        .await?;
        let mut all_users = get_mysql_users(&pool).await?;
        let plugins = fetch_mysql_query(
            self.conns.clone(),
            self.pools.clone(),
            conn_id,
            None,
            "SHOW PLUGINS",
        )
        .await?
        .iter()
        .filter(|row| row.try_get::<String, _>(2).unwrap() == "AUTHENTICATION")
        .map(|row| row.try_get(0).unwrap())
        .collect::<Vec<String>>();

        if let (Some(host_name), Some(user_name)) = (host_name, user_name) {
            let user = get_mysql_user(&pool, host_name, user_name).await?;
            self.old_user = Some(user.clone());

            if matches!(self.db_version, Version::Eight) {
                let member_ofs = get_mysql_user_member_ofs(&pool, host_name, user_name).await?;
                let members = get_mysql_user_members(&pool, host_name, user_name).await?;
                all_users.retain(|u| u.name() != user_name || u.host() != host_name);
                self.member_ofs = all_users
                    .iter()
                    .map(|u| {
                        let same_um = member_ofs.iter().find(|m| {
                            m.member_host.as_ref().unwrap() == user.host()
                                && m.member_name.as_ref().unwrap() == user.name()
                        });
                        UserMember {
                            user_host: Some(u.host().to_string()),
                            user_name: Some(u.name().to_string()),
                            member_host: Some(user.host().to_string()),
                            member_name: Some(user.name().to_string()),
                            granted: same_um.is_some(),
                        }
                    })
                    .collect();

                self.old_member_ofs = self.member_ofs.clone();

                self.members = all_users
                    .iter()
                    .map(|u| {
                        let same_um = members.iter().find(|m| {
                            m.user_host.as_ref().unwrap() == user.host()
                                && m.user_name.as_ref().unwrap() == user.name()
                        });
                        UserMember {
                            user_host: Some(user.host().to_string()),
                            user_name: Some(user.name().to_string()),
                            member_host: Some(u.host().to_string()),
                            member_name: Some(u.name().to_string()),
                            granted: same_um.is_some(),
                        }
                    })
                    .collect();
                self.old_members = self.members.clone();
            }
            self.form.set_items(vec![
                FormItem::new_input("Name".to_string(), Some(user.name()), false, false, false),
                FormItem::new_input("Host".to_string(), Some(user.host()), false, false, false),
                FormItem::new_select(
                    "Plugin".to_string(),
                    plugins,
                    user.plugin().map(|s| s.to_string()),
                    true,
                    false,
                ),
                FormItem::new_input("Password".to_string(), None, true, false, false),
                FormItem::new_input("Confirm Password".to_string(), None, true, false, false),
            ]);
            self.adv_form.set_items(vec![
                FormItem::new_input(
                    "Max queries per hour".to_string(),
                    user.max_queries().as_deref(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "Max updates per hour".to_string(),
                    user.max_updates().as_deref(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "Max connections per hour".to_string(),
                    user.max_connections().as_deref(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "Max user connections".to_string(),
                    user.max_user_connections().as_deref(),
                    true,
                    false,
                    false,
                ),
            ]);

            self.srv_privs.iter_mut().for_each(|(key, val)| {
                let user = self.old_user.as_ref().unwrap();
                match *key {
                    "Alter" => *val = user.alter,
                    "Alter Routine" => *val = user.alter_routine,
                    "Create" => *val = user.create,
                    "Create Routine" => *val = user.create_routine,
                    "Create Temporary Tables" => *val = user.create_temp_tables,
                    "Create User" => *val = user.create_user,
                    "Create View" => *val = user.create_view,
                    "Delete" => *val = user.delete,
                    "Drop" => *val = user.drop,
                    "Event" => *val = user.event,
                    "Execute" => *val = user.execute,
                    "File" => *val = user.file,
                    "Grant Option" => *val = user.grant_option,
                    "Index" => *val = user.index,
                    "Insert" => *val = user.insert,
                    "Lock Tables" => *val = user.lock_tables,
                    "Process" => *val = user.process,
                    "References" => *val = user.references,
                    "Reload" => *val = user.reload,
                    "Replication Client" => *val = user.replication_client,
                    "Replication Slave" => *val = user.replication_slave,
                    "Select" => *val = user.select,
                    "Show Databases" => *val = user.show_databases,
                    "Show View" => *val = user.show_view,
                    "Shutdown" => *val = user.shutdown,
                    "Super" => *val = user.super_priv,
                    "Trigger" => *val = user.trigger,
                    "Update" => *val = user.update,
                    _ => (),
                }
            });
            self.privileges = get_mysql_user_privileges(&pool, host_name, user_name).await?;
            self.old_privileges = self.privileges.clone();
        } else {
            if matches!(self.db_version, Version::Eight) {
                self.member_ofs = all_users
                    .iter()
                    .map(|u| UserMember {
                        user_host: Some(u.host().to_string()),
                        user_name: Some(u.name().to_string()),
                        member_host: None,
                        member_name: None,
                        granted: false,
                    })
                    .collect();
                self.members = all_users
                    .iter()
                    .map(|u| UserMember {
                        user_host: None,
                        user_name: None,
                        member_host: Some(u.host().to_string()),
                        member_name: Some(u.name().to_string()),
                        granted: false,
                    })
                    .collect();
            }
            self.form.set_items(vec![
                FormItem::new_input("Name".to_string(), None, false, false, false),
                FormItem::new_input("Host".to_string(), None, false, false, false),
                FormItem::new_select("Plugin".to_string(), plugins, None, true, false),
                FormItem::new_input("Password".to_string(), None, true, false, false),
                FormItem::new_input("Confirm Password".to_string(), None, true, false, false),
            ]);
            self.adv_form.set_items(vec![
                FormItem::new_input("Max queries per hour".to_string(), None, true, false, false),
                FormItem::new_input("Max updates per hour".to_string(), None, true, false, false),
                FormItem::new_input(
                    "Max connections per hour".to_string(),
                    None,
                    true,
                    false,
                    false,
                ),
                FormItem::new_input("Max user connections".to_string(), None, true, false, false),
            ]);
        }

        Ok(())
    }
    pub fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect, is_focus: bool) -> Result<()>
    where
        B: Backend,
    {
        f.render_widget(
            Block::default()
                .title(if let Some(user) = &self.old_user {
                    format!("Edit User {}", user.name())
                } else {
                    "New User".to_string()
                })
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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(2)].as_ref())
            .split(r.inner(&Margin {
                vertical: 1,
                horizontal: 1,
            }));
        let selected = match self.db_version {
            Version::Eight => match self.panel {
                PanelKind::General => 0,
                PanelKind::Advanced => 1,
                PanelKind::MemberOf => 2,
                PanelKind::Members => 3,
                PanelKind::ServerPrivs => 4,
                PanelKind::Privileges => 5,
                PanelKind::SQLPreview => 6,
            },
            Version::Five => match self.panel {
                PanelKind::General => 0,
                PanelKind::Advanced => 1,
                PanelKind::ServerPrivs => 2,
                PanelKind::Privileges => 3,
                PanelKind::SQLPreview => 4,
                _ => 0,
            },
        };
        let tabs = match self.db_version {
            Version::Eight => vec![
                Spans::from("General"),
                Spans::from("Advanced"),
                Spans::from("Member Of"),
                Spans::from("Memebers"),
                Spans::from("Server Privileges"),
                Spans::from("Privileges"),
                Spans::from("SQL Preview"),
            ],
            Version::Five => vec![
                Spans::from("General"),
                Spans::from("Advanced"),
                Spans::from("Server Privileges"),
                Spans::from("Privileges"),
                Spans::from("SQL Preview"),
            ],
        };
        f.render_widget(
            Tabs::new(tabs)
                .block(Block::default().borders(Borders::BOTTOM))
                .highlight_style(Style::default().fg(Color::Green))
                .select(selected),
            chunks[0],
        );
        match self.panel {
            PanelKind::General => self.draw_general(f, chunks[1]),
            PanelKind::Advanced => self.draw_advanced(f, chunks[1]),
            PanelKind::MemberOf => self.draw_member_ofs(f, chunks[1]),
            PanelKind::Members => self.draw_members(f, chunks[1]),
            PanelKind::ServerPrivs => self.draw_srv_privs(f, chunks[1]),
            PanelKind::Privileges => self.draw_privileges(f, chunks[1]),
            PanelKind::SQLPreview => self.draw_sql_preview(f, chunks[1])?,
        }
        if is_focus {
            self.update_commands();
        }
        Ok(())
    }
    fn draw_general<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(r);
        self.form.draw(f, chunks[0]);
    }
    fn draw_advanced<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(r);
        self.adv_form.draw(f, chunks[0]);
    }
    fn draw_member_ofs<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(self.member_ofs.iter().map(|rm| {
            RowUI::new(vec![
                format!("{}@{}", rm.user_name().unwrap(), rm.user_host().unwrap()),
                String::from(self.bool_str(rm.granted)),
            ])
        }))
        .header(RowUI::new(vec!["User Name", "Granted"]))
        .block(Block::default())
        .widths(&[Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
        .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(table, r, &mut self.member_ofs_state);
    }
    fn draw_members<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(self.members.iter().map(|m| {
            RowUI::new(vec![
                format!("{}@{}", m.member_name().unwrap(), m.member_host().unwrap()),
                String::from(self.bool_str(m.granted)),
            ])
        }))
        .header(RowUI::new(vec!["User Name", "Granted"]))
        .block(Block::default())
        .widths(&[Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
        .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(table, r, &mut self.members_state);
    }
    fn bool_str(&self, val: bool) -> &'static str {
        if val {
            "\u{2705}"
        } else {
            "\u{274E}"
        }
    }
    fn draw_privileges<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(self.privileges.iter().map(|p| {
            RowUI::new([
                p.db.as_str(),
                p.name.as_str(),
                self.bool_str(p.alter),
                self.bool_str(p.create),
                self.bool_str(p.create_view),
                self.bool_str(p.delete),
                self.bool_str(p.drop),
                self.bool_str(p.index),
                self.bool_str(p.insert),
                self.bool_str(p.references),
                self.bool_str(p.select),
                self.bool_str(p.show_view),
                self.bool_str(p.trigger),
                self.bool_str(p.update),
            ])
        }))
        .header(RowUI::new([
            "Database",
            "Name",
            "Alter",
            "Create",
            "Create View",
            "Delete",
            "Drop",
            "Index",
            "Insert",
            "References",
            "Select",
            "Show View",
            "Trigger",
            "Update",
        ]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
            Constraint::Ratio(1, 15),
        ])
        .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(table, r, &mut self.privileges_state);
    }
    fn draw_srv_privs<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(
            self.srv_privs
                .iter()
                .map(|(key, val)| RowUI::new([key, self.bool_str(*val)]))
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(["Privilege", "Granted"]))
        .block(Block::default())
        .widths(&[Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
        .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(table, r, &mut self.srv_priv_state);
    }
    fn draw_sql_preview<B>(&mut self, f: &mut Frame<B>, r: Rect) -> Result<()>
    where
        B: Backend,
    {
        let sql = self.build_sql()?;
        self.sql_preview = TextArea::from(sql.lines());
        f.render_widget(self.sql_preview.widget(), r);
        Ok(())
    }
    pub fn draw_dialog<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        if let Some(dlg) = self.privilege_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.delete_privilege_dlg.as_ref() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.info_dlg.as_ref() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.exit_dlg.as_ref() {
            dlg.draw(f);
        }
    }
    pub async fn handle_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if self.privilege_dlg.is_some() {
            self.handle_privilege_dlg_event(key).await
        } else if self.delete_privilege_dlg.is_some() {
            self.handle_delete_privilege_dlg_event(key)
        } else if self.exit_dlg.is_some() {
            self.handle_exit_dlg_event(key)
        } else if self.info_dlg.is_some() {
            self.handle_info_dlg_event(key)
        } else {
            self.handle_main_event(key).await
        }
    }
    fn handle_exit_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.exit_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => self.exit_dlg = None,
                DialogResult::Confirm(_) => {
                    self.clear();
                    return Ok(ComponentResult::Back(MainPanel::UserListMySQL));
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_info_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.info_dlg.as_mut() {
            dlg.handle_event(key);

            self.old_user = Some(self.get_input_user()?.clone());
            self.old_members = self.members.clone();
            self.old_member_ofs = self.member_ofs.clone();
            self.old_privileges = self.privileges.clone();
            self.info_dlg = None;
        }
        Ok(ComponentResult::Done)
    }
    fn handle_delete_privilege_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.delete_privilege_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_privilege_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.privileges_state.selected() {
                        self.privileges.remove(index);
                        self.delete_privilege_dlg = None;
                        self.privileges_state.select(None);
                    }
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_privilege_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.privilege_dlg.as_mut() {
            match dlg.handle_event(key).await? {
                DialogResult::Cancel => self.privilege_dlg = None,
                DialogResult::Confirm(map) => {
                    let privilege = Privilege {
                        id: if let Some(id) = map.get("id") {
                            if let Some(id) = id {
                                Uuid::parse_str(id).unwrap()
                            } else {
                                Uuid::new_v4()
                            }
                        } else {
                            Uuid::new_v4()
                        },
                        db: map.get("database").unwrap().as_ref().unwrap().to_string(),
                        name: map.get("name").unwrap().as_ref().unwrap().to_string(),
                        alter: map.get("alter").unwrap().as_ref().unwrap() == "true",
                        create: map.get("create").unwrap().as_ref().unwrap() == "true",
                        create_view: map.get("create view").unwrap().as_ref().unwrap() == "true",
                        delete: map.get("delete").unwrap().as_ref().unwrap() == "true",
                        drop: map.get("drop").unwrap().as_ref().unwrap() == "true",
                        index: map.get("index").unwrap().as_ref().unwrap() == "true",
                        insert: map.get("insert").unwrap().as_ref().unwrap() == "true",
                        references: map.get("references").unwrap().as_ref().unwrap() == "true",
                        select: map.get("select").unwrap().as_ref().unwrap() == "true",
                        show_view: map.get("show view").unwrap().as_ref().unwrap() == "true",
                        trigger: map.get("trigger").unwrap().as_ref().unwrap() == "true",
                        update: map.get("update").unwrap().as_ref().unwrap() == "true",
                    };
                    match dlg.get_id() {
                        None => self.privileges.push(privilege),
                        Some(_) => {
                            if let Some(index) = self.privileges_state.selected() {
                                self.privileges.splice(index..index + 1, [privilege]);
                            }
                        }
                    }
                    self.privilege_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_back_event(&mut self) -> Result<ComponentResult> {
        self.exit_dlg = Some(ConfirmDialog::new(
            ConfirmKind::Confirm,
            "Exit",
            "Be sure to exit?",
        ));
        Ok(ComponentResult::Done)
    }
    async fn handle_save_event(&mut self) -> Result<ComponentResult> {
        self.form.validate_input()?;
        self.adv_form.validate_input()?;

        let map = self.form.get_data();
        if map.get("Password") != map.get("Confirm Password") {
            return Err(Error::msg("Password is not same!"));
        }
        let sql = self.build_sql()?;
        let sql = sql.trim();
        if !sql.is_empty() {
            execute_mysql_query_unprepared(
                self.conns.clone(),
                self.pools.clone(),
                &self.conn_id.unwrap(),
                None,
                sql,
            )
            .await?;
            self.info_dlg = Some(ConfirmDialog::new(
                ConfirmKind::Info,
                "Success",
                "Save Success",
            ));
        }

        Ok(ComponentResult::Done)
    }
    async fn handle_main_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match self.panel {
            PanelKind::General => self.handle_panel_general_event(key).await,
            PanelKind::Advanced => self.handle_panel_advanced_event(key).await,
            PanelKind::MemberOf => self.handle_panel_member_of_event(key).await,
            PanelKind::Members => self.handle_panel_members_event(key).await,
            PanelKind::Privileges => self.handle_panel_privileges_event(key).await,
            PanelKind::ServerPrivs => self.handle_panel_server_privileges_event(key).await,
            PanelKind::SQLPreview => self.handle_panel_sql_preview_event(key).await,
        }
    }
    async fn handle_panel_general_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::SQLPreview,
            TAB_RIGHT_KEY => self.panel = PanelKind::Advanced,
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            _ => {
                if let DialogResult::Cancel = self.form.handle_event(key)? {
                    self.handle_back_event()?;
                };
            }
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_advanced_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::General,
            TAB_RIGHT_KEY => match self.db_version {
                Version::Eight => self.panel = PanelKind::MemberOf,
                Version::Five => self.panel = PanelKind::ServerPrivs,
            },
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            _ => {
                if let DialogResult::Cancel = self.adv_form.handle_event(key)? {
                    self.handle_back_event()?;
                }
            }
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_member_of_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::Advanced,
            TAB_RIGHT_KEY => self.panel = PanelKind::Members,
            BACK_KEY => {
                self.handle_back_event()?;
            }
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            UP_KEY => {
                if !self.member_ofs.is_empty() {
                    let index = get_table_up_index(self.member_ofs_state.selected());
                    self.member_ofs_state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.member_ofs.is_empty() {
                    let index = get_table_down_index(
                        self.member_ofs_state.selected(),
                        self.member_ofs.len(),
                    );
                    self.member_ofs_state.select(Some(index));
                }
            }
            CONFIRM_KEY => {
                if let Some(index) = self.member_ofs_state.selected() {
                    self.member_ofs[index].granted = !self.member_ofs[index].granted;
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_members_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::MemberOf,
            TAB_RIGHT_KEY => self.panel = PanelKind::ServerPrivs,
            BACK_KEY => {
                self.handle_back_event()?;
            }
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            UP_KEY => {
                if !self.members.is_empty() {
                    let index = get_table_up_index(self.members_state.selected());
                    self.members_state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.members.is_empty() {
                    let index =
                        get_table_down_index(self.members_state.selected(), self.members.len());
                    self.members_state.select(Some(index));
                }
            }
            CONFIRM_KEY => {
                if let Some(index) = self.members_state.selected() {
                    self.members[index].granted = !self.members[index].granted;
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_server_privileges_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => match self.db_version {
                Version::Eight => self.panel = PanelKind::Members,
                Version::Five => self.panel = PanelKind::Advanced,
            },
            TAB_RIGHT_KEY => self.panel = PanelKind::Privileges,
            BACK_KEY => {
                self.handle_back_event()?;
            }
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            UP_KEY => {
                let index = get_table_up_index(self.srv_priv_state.selected());
                self.srv_priv_state.select(Some(index));
            }
            DOWN_KEY => {
                let index =
                    get_table_down_index(self.srv_priv_state.selected(), self.srv_privs.len());
                self.srv_priv_state.select(Some(index));
            }
            CONFIRM_KEY => {
                if let Some(index) = self.srv_priv_state.selected() {
                    let keys: Vec<&str> = self.srv_privs.keys().cloned().collect();
                    self.srv_privs
                        .entry(keys[index])
                        .and_modify(|val| *val = !*val);
                }
            }
            _ => {}
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_privileges_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::ServerPrivs,
            TAB_RIGHT_KEY => self.panel = PanelKind::SQLPreview,
            BACK_KEY => {
                self.handle_back_event()?;
            }
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            UP_KEY => {
                if !self.privileges.is_empty() {
                    let index = get_table_up_index(self.privileges_state.selected());
                    self.privileges_state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.privileges.is_empty() {
                    let index = get_table_down_index(
                        self.privileges_state.selected(),
                        self.privileges.len(),
                    );
                    self.privileges_state.select(Some(index));
                }
            }
            NEW_KEY => {
                self.privilege_dlg = Some(
                    PrivilegeDialog::new(
                        self.conns.clone(),
                        self.pools.clone(),
                        self.conn_id.unwrap(),
                        None,
                    )
                    .await?,
                );
            }
            CONFIRM_KEY => {
                if let Some(index) = self.privileges_state.selected() {
                    self.privilege_dlg = Some(
                        PrivilegeDialog::new(
                            self.conns.clone(),
                            self.pools.clone(),
                            self.conn_id.unwrap(),
                            Some(&self.privileges[index]),
                        )
                        .await?,
                    );
                }
            }
            DELETE_KEY => {
                self.delete_privilege_dlg = Some(ConfirmDialog::new(
                    ConfirmKind::Warning,
                    "Delete Privilege",
                    "Are you sure to delete privilege",
                ));
            }

            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_sql_preview_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            BACK_KEY => {
                self.handle_back_event()?;
            }
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            TAB_LEFT_KEY => self.panel = PanelKind::Privileges,
            TAB_RIGHT_KEY => self.panel = PanelKind::General,
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    fn clear(&mut self) {
        self.conn_id = None;
        self.db_version = Version::Eight;
        self.old_user = None;
        self.info_dlg = None;
        self.exit_dlg = None;
        self.delete_privilege_dlg = None;
        self.panel = PanelKind::General;
        self.member_ofs = Vec::new();
        self.old_member_ofs = Vec::new();
        self.members = Vec::new();
        self.old_members = Vec::new();
        self.member_ofs_state = TableState::default();
        self.members_state = TableState::default();
        self.srv_privs.iter_mut().for_each(|(_, val)| *val = false);
        self.srv_priv_state = TableState::default();
        self.privileges = Vec::new();
        self.old_privileges = Vec::new();
        self.privileges_state = TableState::default();
        self.sql_preview = TextArea::default();
        self.form.clear();
        self.adv_form.clear();
        self.privilege_dlg = None;
    }

    fn build_sql(&self) -> Result<String> {
        if self.old_user.is_some() {
            self.build_alter_ddl()
        } else {
            Ok(self.build_create_ddl())
        }
    }
    fn build_alter_ddl(&self) -> Result<String> {
        let mut ddls = Vec::new();

        let user = self.get_input_user()?;

        let user_ddl = self.build_user_ddl(&user)?;
        ddls.extend(user_ddl);
        let member_of_ddl = self.build_member_ofs_ddl(&user);
        ddls.extend(member_of_ddl);
        let members_ddl = self.build_members_ddl(&user);
        ddls.extend(members_ddl);
        let privileges_ddl = self.build_privileges_ddl(&user);
        ddls.extend(privileges_ddl);

        Ok(ddls.join("\n"))
    }
    fn build_user_ddl(&self, user: &User) -> Result<Vec<String>> {
        let mut ddl = Vec::new();

        let old_user = self.old_user.as_ref().unwrap();
        if user.name() != old_user.name() || user.host() != old_user.host() {
            ddl.push(format!(
                "RENAME USER `{}`@`{}` TO `{}`@`{}`",
                old_user.name(),
                old_user.host(),
                user.name(),
                user.host()
            ));
        }
        let pwd_ddl = if let Some(pwd) = user.password() {
            if !pwd.is_empty() {
                format!(" BY '{}'", pwd)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let plugin_ddl = if user.plugin() != old_user.plugin() && user.plugin().is_some() {
            if let Some(plugin) = user.plugin() {
                if !plugin.is_empty() {
                    format!(" WITH {}", plugin)
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let identity_ddl = if !pwd_ddl.is_empty() || !plugin_ddl.is_empty() {
            format!("IDENTIFIED{}{}", plugin_ddl, pwd_ddl)
        } else {
            String::new()
        };
        let mut resource_ddl = Vec::new();
        if user.max_queries() != old_user.max_queries() {
            if let Some(mqph) = user.max_queries() {
                if !mqph.is_empty() {
                    resource_ddl.push(format!("MAX_QUERIES_PER_HOUR {}", mqph));
                }
            }
        }
        if user.max_updates() != old_user.max_updates() {
            if let Some(mupr) = user.max_updates() {
                if !mupr.is_empty() {
                    resource_ddl.push(format!("MAX_UPDATES_PER_HOUR {}", mupr));
                }
            }
        }
        if user.max_connections() != old_user.max_connections() {
            if let Some(mcph) = user.max_connections() {
                if !mcph.is_empty() {
                    resource_ddl.push(format!("MAX_CONNECTIONS_PER_HOUR {}", mcph));
                }
            }
        }
        if user.max_user_connections() != old_user.max_user_connections() {
            if let Some(muc) = user.max_user_connections() {
                if !muc.is_empty() {
                    resource_ddl.push(format!("MAX_USER_CONNECTIONS {}", muc));
                }
            }
        }
        if !pwd_ddl.is_empty() || !resource_ddl.is_empty() {
            ddl.push(format!(
                "ALTER USER `{}`@`{}` {}{};",
                user.name(),
                user.host(),
                identity_ddl,
                if !resource_ddl.is_empty() {
                    format!(" WITH {}", resource_ddl.join("\n"))
                } else {
                    String::new()
                },
            ));
        }
        let mut srv_grant_ddl = Vec::new();
        let mut srv_revoke_ddl = Vec::new();
        self.srv_privs.iter().for_each(|(key, _)| match *key {
            "Alter" => {
                if user.alter != old_user.alter {
                    if user.alter {
                        srv_grant_ddl.push("Alter");
                    } else {
                        srv_revoke_ddl.push("Alter");
                    }
                }
            }
            "Alter Routine" => {
                if user.alter_routine != old_user.alter_routine {
                    if user.alter_routine {
                        srv_grant_ddl.push("Alter Routine");
                    } else {
                        srv_revoke_ddl.push("Alter Routine");
                    }
                }
            }
            "Create" => {
                if user.create != old_user.create {
                    if user.create {
                        srv_grant_ddl.push("Create");
                    } else {
                        srv_revoke_ddl.push("Create");
                    }
                }
            }
            "Create Routine" => {
                if user.create_routine != old_user.create_routine {
                    if user.create_routine {
                        srv_grant_ddl.push("Create Routine");
                    } else {
                        srv_revoke_ddl.push("Create Routine");
                    }
                }
            }
            "Create Temporary Tables" => {
                if user.create_temp_tables != old_user.create_temp_tables {
                    if user.create_temp_tables {
                        srv_grant_ddl.push("Create Temporary Tables");
                    } else {
                        srv_revoke_ddl.push("Create Temporary Tables");
                    }
                }
            }
            "Create User" => {
                if user.create_user != old_user.create_user {
                    if user.create_user {
                        srv_grant_ddl.push("Create User");
                    } else {
                        srv_revoke_ddl.push("Create User");
                    }
                }
            }
            "Create View" => {
                if user.create_view != old_user.create_view {
                    if user.create_view {
                        srv_grant_ddl.push("Create View");
                    } else {
                        srv_revoke_ddl.push("Create View");
                    }
                }
            }
            "Delete" => {
                if user.delete != old_user.delete {
                    if user.delete {
                        srv_grant_ddl.push("Delete");
                    } else {
                        srv_revoke_ddl.push("Delete");
                    }
                }
            }
            "Drop" => {
                if user.drop != old_user.drop {
                    if user.drop {
                        srv_grant_ddl.push("Drop");
                    } else {
                        srv_revoke_ddl.push("Drop");
                    }
                }
            }
            "Event" => {
                if user.event != old_user.event {
                    if user.event {
                        srv_grant_ddl.push("Event");
                    } else {
                        srv_revoke_ddl.push("Event");
                    }
                }
            }
            "Execute" => {
                if user.execute != old_user.execute {
                    if user.execute {
                        srv_grant_ddl.push("Execute");
                    } else {
                        srv_revoke_ddl.push("Execute");
                    }
                }
            }
            "File" => {
                if user.file != old_user.file {
                    if user.file {
                        srv_grant_ddl.push("File");
                    } else {
                        srv_revoke_ddl.push("File");
                    }
                }
            }
            "Grant Option" => {
                if user.grant_option != old_user.grant_option {
                    if user.grant_option {
                        srv_grant_ddl.push("Grant Option");
                    } else {
                        srv_revoke_ddl.push("Grant Option");
                    }
                }
            }
            "Index" => {
                if user.index != old_user.index {
                    if user.index {
                        srv_grant_ddl.push("Index");
                    } else {
                        srv_revoke_ddl.push("Index");
                    }
                }
            }
            "Insert" => {
                if user.insert != old_user.insert {
                    if user.insert {
                        srv_grant_ddl.push("Insert");
                    } else {
                        srv_revoke_ddl.push("Insert");
                    }
                }
            }
            "Lock Tables" => {
                if user.lock_tables != old_user.lock_tables {
                    if user.lock_tables {
                        srv_grant_ddl.push("Lock Tables");
                    } else {
                        srv_revoke_ddl.push("Locak Tables");
                    }
                }
            }
            "Process" => {
                if user.process != old_user.process {
                    if user.process {
                        srv_grant_ddl.push("Process");
                    } else {
                        srv_revoke_ddl.push("Process");
                    }
                }
            }
            "References" => {
                if user.references != old_user.references {
                    if user.references {
                        srv_grant_ddl.push("References");
                    } else {
                        srv_revoke_ddl.push("References");
                    }
                }
            }
            "Reload" => {
                if user.reload != old_user.reload {
                    if user.reload {
                        srv_grant_ddl.push("Reload");
                    } else {
                        srv_revoke_ddl.push("Reload");
                    }
                }
            }
            "Replication Client" => {
                if user.replication_client != old_user.replication_client {
                    if user.replication_client {
                        srv_grant_ddl.push("Replication Client");
                    } else {
                        srv_revoke_ddl.push("Replication Client");
                    }
                }
            }
            "Replication Slave" => {
                if user.replication_slave != old_user.replication_slave {
                    if user.replication_slave {
                        srv_grant_ddl.push("Replication Slave");
                    } else {
                        srv_revoke_ddl.push("Replication Slave");
                    }
                }
            }
            "Select" => {
                if user.select != old_user.select {
                    if user.select {
                        srv_grant_ddl.push("Select");
                    } else {
                        srv_revoke_ddl.push("Select");
                    }
                }
            }
            "Show Databases" => {
                if user.show_databases != old_user.show_databases {
                    if user.show_databases {
                        srv_grant_ddl.push("Show Databases");
                    } else {
                        srv_revoke_ddl.push("Show Databases");
                    }
                }
            }
            "Show View" => {
                if user.show_view != old_user.show_view {
                    if user.show_view {
                        srv_grant_ddl.push("Show View");
                    } else {
                        srv_revoke_ddl.push("Show View");
                    }
                }
            }
            "Shutdown" => {
                if user.shutdown != old_user.shutdown {
                    if user.shutdown {
                        srv_grant_ddl.push("Shutdown");
                    } else {
                        srv_revoke_ddl.push("Shutdown");
                    }
                }
            }
            "Super" => {
                if user.super_priv != old_user.super_priv {
                    if user.super_priv {
                        srv_grant_ddl.push("Super");
                    } else {
                        srv_revoke_ddl.push("Super");
                    }
                }
            }
            "Trigger" => {
                if user.trigger != old_user.trigger {
                    if user.trigger {
                        srv_grant_ddl.push("Trigger");
                    } else {
                        srv_revoke_ddl.push("Trigger");
                    }
                }
            }
            "Update" => {
                if user.update != old_user.update {
                    if user.update {
                        srv_grant_ddl.push("Update");
                    } else {
                        srv_revoke_ddl.push("Update");
                    }
                }
            }
            _ => (),
        });
        if !srv_grant_ddl.is_empty() {
            ddl.push(format!(
                "GRANT {} ON *.* TO `{}`@`{}`;",
                srv_grant_ddl.join(","),
                user.name(),
                user.host()
            ));
        }
        if !srv_revoke_ddl.is_empty() {
            ddl.push(format!(
                "REVOKE {} ON *.* FROM `{}`@`{}`;",
                srv_revoke_ddl.join(","),
                user.name(),
                user.host()
            ));
        }

        Ok(ddl)
    }
    fn get_srv_priv(&self, key: &str) -> bool {
        *self.srv_privs.get(key).unwrap()
    }
    fn get_input_user(&self) -> Result<User> {
        let map = self.form.get_data();
        let adv_map = self.adv_form.get_data();

        Ok(User {
            host: map
                .get("Host")
                .unwrap()
                .as_ref()
                .map(|host| host.to_string())
                .unwrap(),
            name: map
                .get("Name")
                .unwrap()
                .as_ref()
                .map(|name| name.to_string())
                .unwrap(),
            plugin: map.get("Plugin").unwrap().as_ref().map(|s| s.to_string()),
            password: map
                .get("Password")
                .unwrap()
                .as_ref()
                .map(|pwd| pwd.to_string()),
            max_queries: adv_map
                .get("Max queries per hour")
                .unwrap()
                .as_ref()
                .map(|mq| mq.parse().unwrap_or(0)),
            max_updates: adv_map
                .get("Max updates per hour")
                .unwrap()
                .as_ref()
                .map(|mu| mu.parse().unwrap_or(0)),
            max_connections: adv_map
                .get("Max connections per hour")
                .unwrap()
                .as_ref()
                .map(|mc| mc.parse().unwrap_or(0)),
            max_user_connections: adv_map
                .get("Max user connections")
                .unwrap()
                .as_ref()
                .map(|muc| muc.parse().unwrap_or(0)),
            alter: self.get_srv_priv("Alter"),
            alter_routine: self.get_srv_priv("Alter Routine"),
            create: self.get_srv_priv("Create"),
            create_routine: self.get_srv_priv("Create Routine"),
            create_temp_tables: self.get_srv_priv("Create Temporary Tables"),
            create_user: self.get_srv_priv("Create User"),
            create_view: self.get_srv_priv("Create View"),
            delete: self.get_srv_priv("Delete"),
            drop: self.get_srv_priv("Drop"),
            event: self.get_srv_priv("Event"),
            execute: self.get_srv_priv("Execute"),
            file: self.get_srv_priv("File"),
            grant_option: self.get_srv_priv("Grant Option"),
            index: self.get_srv_priv("Index"),
            insert: self.get_srv_priv("Insert"),
            lock_tables: self.get_srv_priv("Lock Tables"),
            process: self.get_srv_priv("Process"),
            references: self.get_srv_priv("References"),
            reload: self.get_srv_priv("Reload"),
            replication_client: self.get_srv_priv("Replication Client"),
            replication_slave: self.get_srv_priv("Replication Slave"),
            select: self.get_srv_priv("Select"),
            show_databases: self.get_srv_priv("Show Databases"),
            show_view: self.get_srv_priv("Show View"),
            shutdown: self.get_srv_priv("Shutdown"),
            super_priv: self.get_srv_priv("Super"),
            trigger: self.get_srv_priv("Trigger"),
            update: self.get_srv_priv("Update"),
        })
    }
    fn build_member_ofs_ddl(&self, user: &User) -> Vec<String> {
        let mut ddls = Vec::new();

        self.member_ofs.iter().for_each(|mo| {
            let same_mo = self
                .old_member_ofs
                .iter()
                .find(|old_mo| mo.user_host == old_mo.user_host && mo.user_name == old_mo.user_name)
                .unwrap();
            if let Some(ddl) = mo.get_alter_ddl(same_mo, user.name(), user.host()) {
                ddls.push(ddl);
            }
        });
        ddls
    }
    fn build_members_ddl(&self, user: &User) -> Vec<String> {
        let mut ddls = Vec::new();
        self.members.iter().for_each(|mo| {
            let same_mo = self
                .old_members
                .iter()
                .find(|old_mo| {
                    mo.user_host == old_mo.user_host
                        && mo.user_name == old_mo.user_name
                        && mo.member_host == old_mo.member_host
                        && mo.member_name == old_mo.member_name
                })
                .unwrap();
            if let Some(ddl) = mo.get_alter_ddl(same_mo, user.name(), user.host()) {
                ddls.push(ddl);
            }
        });
        ddls
    }
    fn build_privileges_ddl(&self, user: &User) -> Vec<String> {
        let mut ddls = Vec::new();
        let privilege_ids = self.privileges.iter().map(|p| p.id).collect::<Vec<Uuid>>();
        let old_privilege_ids = self
            .old_privileges
            .iter()
            .map(|p| p.id)
            .collect::<Vec<Uuid>>();
        let mut revoke_all_ddl: Vec<String> = self
            .old_privileges
            .iter()
            .filter(|p| !privilege_ids.contains(&p.id))
            .map(|p| p.get_revoke_all_ddl(user.name(), user.host()))
            .collect();
        ddls.append(&mut revoke_all_ddl);
        self.privileges.iter().for_each(|privilege| {
            if !old_privilege_ids.contains(&privilege.id) {
                let grant_ddl = privilege.get_grant_ddl(user.name(), user.host());
                ddls.push(grant_ddl);
            } else {
                let same_privilege = self
                    .old_privileges
                    .iter()
                    .find(|p| p.id == privilege.id)
                    .unwrap();
                let mut alter_ddls =
                    privilege.get_alter_ddl(same_privilege, user.name(), user.host());
                ddls.append(&mut alter_ddls);
            }
        });
        ddls
    }
    fn build_create_ddl(&self) -> String {
        let map = self.form.get_data();
        let adv_map = self.adv_form.get_data();
        let default_name = String::from("username");
        let default_host = String::from("localhost");
        let name = map
            .get("Name")
            .unwrap()
            .as_ref()
            .map(|name| name.to_string())
            .unwrap_or(default_name.clone());
        let host = map
            .get("Host")
            .unwrap()
            .as_ref()
            .map(|host| host.to_string())
            .unwrap_or(default_host.clone());
        let member_ofs_ddl: Vec<String> = self
            .member_ofs
            .iter()
            .filter(|mo| mo.granted)
            .map(|mo| {
                format!(
                    "GRANT `{}`@`{}` TO `{}`@`{}`;",
                    mo.user_name.as_ref().unwrap(),
                    mo.user_host.as_ref().unwrap(),
                    name,
                    host,
                )
            })
            .collect();
        let members_ddl: Vec<String> = self
            .members
            .iter()
            .filter(|ms| ms.granted)
            .map(|ms| {
                format!(
                    "GRANT `{}`@`{}` TO `{}`@`{}`;",
                    name,
                    host,
                    ms.member_name.as_ref().unwrap(),
                    ms.member_host.as_ref().unwrap(),
                )
            })
            .collect();

        let mut resource_opts = Vec::new();
        if let Some(mqph) = adv_map.get("Max queries per hour").unwrap() {
            if !mqph.is_empty() {
                resource_opts.push(format!("MAX_QUERIES_PER_HOUR {}", mqph));
            }
        }
        if let Some(mupr) = adv_map.get("Max updates per hour").unwrap() {
            if !mupr.is_empty() {
                resource_opts.push(format!("MAX_UPDATES_PER_HOUR {}", mupr));
            }
        }
        if let Some(mcph) = adv_map.get("Max connections per hour").unwrap() {
            if !mcph.is_empty() {
                resource_opts.push(format!("MAX_CONNECTIONS_PER_HOUR {}", mcph));
            }
        }
        if let Some(muc) = adv_map.get("Max user connections").unwrap() {
            if !muc.is_empty() {
                resource_opts.push(format!("MAX_USER_CONNECTIONS {}", muc));
            }
        }
        let pwd_ddl = if let Some(pwd) = map.get("Password").unwrap() {
            if !pwd.is_empty() {
                format!(" BY '{}'", pwd)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let plugin_ddl = if let Some(plugin) = map.get("Plugin").unwrap() {
            if !plugin.is_empty() {
                format!(" WITH {}", plugin)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let identity_ddl = if !pwd_ddl.is_empty() || !plugin_ddl.is_empty() {
            format!(" IDENTIFIED{}{}", plugin_ddl, pwd_ddl)
        } else {
            String::new()
        };

        let user_ddl = format!(
            "CREATE USER `{}`@`{}`{}{};",
            name,
            host,
            identity_ddl,
            if !resource_opts.is_empty() {
                format!(" WITH {}", resource_opts.join("\n"))
            } else {
                String::new()
            },
        );
        let privs_ddl: Vec<String> = self
            .privileges
            .iter()
            .map(|p| {
                let mut p_str = Vec::new();
                if p.alter {
                    p_str.push("Alter");
                }
                if p.create {
                    p_str.push("Create");
                }
                if p.create_view {
                    p_str.push("Create View");
                }
                if p.delete {
                    p_str.push("Delete");
                }
                if p.drop {
                    p_str.push("Drop");
                }
                if p.index {
                    p_str.push("Index");
                }
                if p.insert {
                    p_str.push("Insert");
                }
                if p.references {
                    p_str.push("References");
                }
                if p.select {
                    p_str.push("Select");
                }
                if p.show_view {
                    p_str.push("Show View");
                }
                if p.trigger {
                    p_str.push("Trigger");
                }
                if p.update {
                    p_str.push("Update");
                }
                format!(
                    "GRANT {} ON `{}`.`{}` TO `{}`@`{}`;",
                    p_str.join(","),
                    p.db,
                    p.name,
                    name,
                    host
                )
            })
            .collect();
        let srv_privs: Vec<&str> = self
            .srv_privs
            .iter()
            .filter(|(_, val)| **val)
            .map(|(key, _)| *key)
            .collect();

        let srv_privs_ddl = if !srv_privs.is_empty() {
            format!(
                "GRANT {} ON *.* TO `{}`@`{}`;",
                srv_privs.join(","),
                name,
                host
            )
        } else {
            String::new()
        };
        format!(
            "{}\n{}\n{}\n{}\n{}\n",
            user_ddl,
            member_ofs_ddl.join("\n"),
            members_ddl.join("\n"),
            srv_privs_ddl,
            privs_ddl.join("\n"),
        )
    }

    fn update_commands(&self) {
        let mut cmds = if let Some(dlg) = self.privilege_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.delete_privilege_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.exit_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.info_dlg.as_ref() {
            dlg.get_commands()
        } else {
            self.get_main_commands()
        };
        self.cmd_bar.borrow_mut().set_commands(&mut cmds);
    }
    fn get_main_commands(&self) -> Vec<Command> {
        let mut cmds = match self.panel {
            PanelKind::General => self.get_panel_general_commands(),
            PanelKind::Advanced => self.get_panel_advanced_commands(),
            PanelKind::MemberOf => self.get_panel_member_ofs_commands(),
            PanelKind::Members => self.get_panel_members_commands(),
            PanelKind::ServerPrivs => self.get_panel_server_privileges_commands(),
            PanelKind::Privileges => self.get_panel_privileges_commands(),
            PanelKind::SQLPreview => self.get_panel_sql_preview_commands(),
        };
        cmds.extend(vec![
            Command {
                name: "Save User",
                key: SAVE_KEY,
            },
            Command {
                name: "Back to Users",
                key: BACK_KEY,
            },
        ]);
        cmds
    }
    fn get_panel_general_commands(&self) -> Vec<Command> {
        let mut cmds = self.form.get_commands();
        cmds.extend(vec![
            Command {
                name: "Next Panel",
                key: TAB_RIGHT_KEY,
            },
            Command {
                name: "Previous Panel",
                key: TAB_LEFT_KEY,
            },
        ]);
        cmds
    }
    fn get_panel_advanced_commands(&self) -> Vec<Command> {
        let mut cmds = self.adv_form.get_commands();
        cmds.extend(vec![
            Command {
                name: "Next Panel",
                key: TAB_RIGHT_KEY,
            },
            Command {
                name: "Previous Panel",
                key: TAB_LEFT_KEY,
            },
        ]);
        cmds
    }
    fn get_panel_member_ofs_commands(&self) -> Vec<Command> {
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
                name: "Next Panel",
                key: TAB_RIGHT_KEY,
            },
            Command {
                name: "Previous Panel",
                key: TAB_LEFT_KEY,
            },
        ];
        if self.member_ofs_state.selected().is_some() {
            cmds.push(Command {
                name: "Toggle Granted",
                key: CONFIRM_KEY,
            });
        }
        cmds
    }
    fn get_panel_members_commands(&self) -> Vec<Command> {
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
                name: "Next Panel",
                key: TAB_RIGHT_KEY,
            },
            Command {
                name: "Previous Panel",
                key: TAB_LEFT_KEY,
            },
        ];
        if self.members_state.selected().is_some() {
            cmds.push(Command {
                name: "Toggle Granted",
                key: CONFIRM_KEY,
            });
        }
        cmds
    }
    fn get_panel_server_privileges_commands(&self) -> Vec<Command> {
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
                name: "Next Panel",
                key: TAB_RIGHT_KEY,
            },
            Command {
                name: "Previous Panel",
                key: TAB_LEFT_KEY,
            },
        ];
        if self.srv_priv_state.selected().is_some() {
            cmds.push(Command {
                name: "Toggle Value",
                key: CONFIRM_KEY,
            });
        }
        cmds
    }
    fn get_panel_privileges_commands(&self) -> Vec<Command> {
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
                name: "Next Panel",
                key: TAB_RIGHT_KEY,
            },
            Command {
                name: "Previous Panel",
                key: TAB_LEFT_KEY,
            },
            Command {
                name: "Add Privilege",
                key: NEW_KEY,
            },
        ];
        if self.privileges_state.selected().is_some() {
            cmds.push(Command {
                name: "Open Privilege",
                key: CONFIRM_KEY,
            });
            cmds.push(Command {
                name: "Delete Privilege",
                key: DELETE_KEY,
            });
        }
        cmds
    }
    fn get_panel_sql_preview_commands(&self) -> Vec<Command> {
        vec![
            Command {
                name: "Next Panel",
                key: TAB_RIGHT_KEY,
            },
            Command {
                name: "Previous Panel",
                key: TAB_LEFT_KEY,
            },
        ]
    }
}
