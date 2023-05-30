use crate::{
    app::{ComponentResult, DialogResult, MainPanel},
    component::{get_table_down_index, get_table_up_index, Command, CommandBarComponent},
    dialog::{
        confirm::Kind as ConfirmKind,
        pg::{PrivilegeDialog, RoleMemberDialog},
        ConfirmDialog, InputDialog,
    },
    event::{config::*, Key},
    model::pg::{
        get_pg_role, get_pg_role_member_ofs, get_pg_role_members, get_pg_role_privileges,
        get_pg_roles, Connections, Privilege, Role, RoleMember,
    },
    pool::{execute_pg_query_unprepared, get_pg_pool, PGPools},
    widget::{Form, FormItem},
};
use anyhow::Result;
use chrono::{Local, NaiveDateTime, TimeZone};
use sqlx::postgres::types::Oid;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Row as RowUI, Table, TableState, Tabs},
    Frame,
};
use tui_textarea::{Input, TextArea};
use uuid::Uuid;

enum PanelKind {
    General,
    MemberOf,
    Members,
    Privileges,
    Comment,
    SQLPreview,
}

pub struct RoleDetailComponent<'a> {
    conn_id: Option<Uuid>,
    role: Option<Role>,
    input_dlg: Option<InputDialog<'a>>,
    info_dlg: Option<ConfirmDialog>,
    exit_dlg: Option<ConfirmDialog>,
    panel: PanelKind,
    member_ofs: Vec<RoleMember>,
    old_member_ofs: Vec<RoleMember>,
    member_ofs_state: TableState,
    members: Vec<RoleMember>,
    old_members: Vec<RoleMember>,
    members_state: TableState,
    privileges: Vec<Privilege>,
    old_privileges: Vec<Privilege>,
    privileges_state: TableState,
    comment: TextArea<'a>,
    sql_preview: TextArea<'a>,
    form: Form<'a>,
    member_of_dlg: Option<RoleMemberDialog<'a>>,
    members_dlg: Option<RoleMemberDialog<'a>>,
    privilege_dlg: Option<PrivilegeDialog<'a>>,
    delete_privilege_dlg: Option<ConfirmDialog>,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<PGPools>>,
    cmd_bar: Rc<RefCell<CommandBarComponent>>,
}

impl<'a> RoleDetailComponent<'a> {
    pub fn new(
        cmd_bar: Rc<RefCell<CommandBarComponent>>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<PGPools>>,
    ) -> Self {
        RoleDetailComponent {
            conn_id: None,
            role: None,
            input_dlg: None,
            info_dlg: None,
            exit_dlg: None,
            form: Form::default(),
            panel: PanelKind::General,
            member_ofs: Vec::new(),
            old_member_ofs: Vec::new(),
            member_ofs_state: TableState::default(),
            members: Vec::new(),
            old_members: Vec::new(),
            members_state: TableState::default(),
            privileges: Vec::new(),
            old_privileges: Vec::new(),
            privileges_state: TableState::default(),
            delete_privilege_dlg: None,
            comment: TextArea::default(),
            sql_preview: TextArea::default(),
            member_of_dlg: None,
            members_dlg: None,
            privilege_dlg: None,
            conns,
            pools,
            cmd_bar,
        }
    }
    pub async fn set_data(&mut self, conn_id: &Uuid, role_name: Option<&str>) -> Result<()> {
        self.conn_id = Some(*conn_id);
        let pool = get_pg_pool(
            self.conns.clone(),
            self.pools.clone(),
            self.conn_id.as_ref().unwrap(),
            None,
        )
        .await?;
        let roles = get_pg_roles(&pool).await?;

        if let Some(name) = role_name {
            let role = get_pg_role(&pool, name).await?;
            self.role = Some(role.clone());

            let member_ofs = get_pg_role_member_ofs(&pool, name).await?;
            let members = get_pg_role_members(&pool, name).await?;

            self.member_ofs = roles
                .iter()
                .map(|r| {
                    let same_mo = member_ofs.iter().find(|m| {
                        m.role_oid.as_ref().unwrap() == r.oid()
                            && m.member_oid.as_ref().unwrap() == role.oid()
                    });

                    RoleMember {
                        role_oid: Some(r.oid().to_owned()),
                        role_name: Some(r.name().to_owned()),
                        member_oid: Some(role.oid().to_owned()),
                        member_name: Some(role.name().to_owned()),
                        granted: same_mo.is_some(),
                        admin_option: if let Some(m) = same_mo {
                            m.admin_option
                        } else {
                            false
                        },
                    }
                })
                .collect();
            self.old_member_ofs = self.member_ofs.clone();
            self.members = roles
                .iter()
                .map(|r| {
                    let same_ms = members.iter().find(|m| {
                        m.role_oid.as_ref().unwrap() == role.oid()
                            && m.member_oid.as_ref().unwrap() == r.oid()
                    });

                    RoleMember {
                        role_oid: Some(role.oid().to_owned()),
                        role_name: Some(role.name().to_owned()),
                        member_oid: Some(r.oid().to_owned()),
                        member_name: Some(r.name().to_owned()),
                        granted: same_ms.is_some(),
                        admin_option: if let Some(m) = same_ms {
                            m.admin_option
                        } else {
                            false
                        },
                    }
                })
                .collect();
            self.old_members = self.members.clone();
            self.form.set_items(vec![
                FormItem::new_input("name".to_string(), Some(role.name()), false, false, false),
                FormItem::new_check("can login".to_string(), role.can_login(), false),
                FormItem::new_input("password".to_string(), None, true, false, false),
                FormItem::new_input(
                    "connect limit".to_string(),
                    Some(role.conn_limit().to_string().as_str()),
                    false,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "expiry date".to_string(),
                    role.expiry_date()
                        .map(|ed| ed.format("%Y-%m-%d %H:%M:%S").to_string())
                        .as_deref(),
                    false,
                    false,
                    false,
                ),
                FormItem::new_check("superuser".to_string(), role.super_user(), false),
                FormItem::new_check("can create databases".to_string(), role.create_db(), false),
                FormItem::new_check("can create roles".to_string(), role.create_role(), false),
                FormItem::new_check("inherit privileges".to_string(), role.inherit(), false),
                FormItem::new_check("can replicate".to_string(), role.replication(), false),
                FormItem::new_check("can bypass rls".to_string(), role.bypassrls(), false),
            ]);
            self.member_ofs.iter_mut().for_each(|mo| {
                let same_mo = member_ofs
                    .iter()
                    .find(|m| m.role_oid == mo.role_oid && m.member_oid == mo.member_oid);
                if let Some(m) = same_mo {
                    mo.granted = true;
                    mo.admin_option = m.admin_option;
                }
            });

            self.privileges = get_pg_role_privileges(&pool, name).await?;
            self.old_privileges = self.privileges.clone();
        } else {
            self.member_ofs = roles
                .iter()
                .map(|r| RoleMember {
                    role_oid: Some(r.oid().to_owned()),
                    role_name: Some(r.name().to_owned()),
                    member_oid: None,
                    member_name: None,
                    granted: false,
                    admin_option: false,
                })
                .collect();
            self.members = roles
                .iter()
                .map(|r| RoleMember {
                    role_oid: None,
                    role_name: None,
                    member_oid: Some(r.oid().to_owned()),
                    member_name: Some(r.name().to_owned()),
                    granted: false,
                    admin_option: false,
                })
                .collect();

            self.form.set_items(vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_check("can login".to_string(), false, false),
                FormItem::new_input("password".to_string(), None, true, false, false),
                FormItem::new_input("connect limit".to_string(), Some("-1"), false, false, false),
                FormItem::new_input("expiry date".to_string(), None, false, false, false),
                FormItem::new_check("superuser".to_string(), false, false),
                FormItem::new_check("can create databases".to_string(), false, false),
                FormItem::new_check("can create roles".to_string(), false, false),
                FormItem::new_check("inherit privileges".to_string(), false, false),
                FormItem::new_check("can replicate".to_string(), false, false),
                FormItem::new_check("can bypass rls".to_string(), false, false),
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
                .title(if let Some(role) = &self.role {
                    format!("Edit Role {}", role.name())
                } else {
                    "New Role".to_string()
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
        let selected = match self.panel {
            PanelKind::General => 0,
            PanelKind::MemberOf => 1,
            PanelKind::Members => 2,
            PanelKind::Privileges => 3,
            PanelKind::Comment => 4,
            PanelKind::SQLPreview => 5,
        };
        f.render_widget(
            Tabs::new(
                [
                    Span::raw("General"),
                    Span::raw("Member Of"),
                    Span::raw("Members"),
                    Span::raw("Privileges"),
                    Span::raw("Comment"),
                    Span::raw("SQL Preview"),
                ]
                .iter()
                .cloned()
                .map(Spans::from)
                .collect(),
            )
            .block(Block::default().borders(Borders::BOTTOM))
            .highlight_style(Style::default().fg(Color::Green))
            .select(selected),
            chunks[0],
        );
        match self.panel {
            PanelKind::General => self.draw_general(f, chunks[1]),
            PanelKind::Privileges => self.draw_privileges(f, chunks[1]),
            PanelKind::MemberOf => self.draw_member_of(f, chunks[1]),
            PanelKind::Members => self.draw_members(f, chunks[1]),
            PanelKind::Comment => self.draw_comment(f, chunks[1]),
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
                p.schema.as_str(),
                p.name.as_str(),
                self.bool_str(p.delete),
                self.bool_str(p.insert),
                self.bool_str(p.references),
                self.bool_str(p.select),
                self.bool_str(p.trigger),
                self.bool_str(p.truncate),
                self.bool_str(p.update),
            ])
        }))
        .header(RowUI::new([
            "Database",
            "Schema",
            "Name",
            "Delete",
            "Insert",
            "References",
            "Select",
            "Trigger",
            "Truncate",
            "Update",
        ]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
            Constraint::Ratio(1, 10),
        ])
        .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(table, r, &mut self.privileges_state);
    }
    fn draw_member_of<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(self.member_ofs.iter().map(|rm| {
            RowUI::new(vec![
                rm.role_name.as_deref().unwrap(),
                if rm.granted { "\u{2705}" } else { "\u{274E}" },
                if rm.admin_option {
                    "\u{2705}"
                } else {
                    "\u{274E}"
                },
            ])
        }))
        .header(RowUI::new(vec!["Role Name", "Granted", "Admin Option"]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(table, r, &mut self.member_ofs_state);
    }
    fn draw_members<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(self.members.iter().map(|m| {
            RowUI::new(vec![
                m.member_name.as_deref().unwrap(),
                if m.granted { "\u{2705}" } else { "\u{274E}" },
                if m.admin_option {
                    "\u{2705}"
                } else {
                    "\u{274E}"
                },
            ])
        }))
        .header(RowUI::new(vec!["Role Name", "Granted", "Admin Option"]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(table, r, &mut self.members_state);
    }
    fn draw_comment<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        f.render_widget(self.comment.widget(), r);
    }
    fn draw_sql_preview<B>(&mut self, f: &mut Frame<B>, r: Rect) -> Result<()>
    where
        B: Backend,
    {
        let sql = self.build_sql(None)?;
        self.sql_preview = TextArea::from(sql.lines());
        f.render_widget(self.sql_preview.widget(), r);
        Ok(())
    }
    pub fn draw_dialog<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        if let Some(dlg) = self.member_of_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.members_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.privilege_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.input_dlg.as_ref() {
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
        if self.member_of_dlg.is_some() {
            self.handle_member_of_dlg_event(key)
        } else if self.members_dlg.is_some() {
            self.handle_members_dlg_event(key)
        } else if self.privilege_dlg.is_some() {
            self.handle_privilege_dlg_event(key).await
        } else if self.input_dlg.is_some() {
            self.handle_input_dlg_event(key).await
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
                    return Ok(ComponentResult::Back(MainPanel::RoleListPG));
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_info_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.info_dlg.as_mut() {
            dlg.handle_event(key);
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
                    }
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_member_of_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.member_of_dlg.as_mut() {
            match dlg.handle_event(key)? {
                DialogResult::Cancel => self.member_of_dlg = None,
                DialogResult::Confirm(map) => {
                    let role_oid = map
                        .get("role_oid")
                        .unwrap()
                        .as_ref()
                        .map(|ro| Oid(ro.parse().unwrap()));
                    let member_oid = map
                        .get("member_oid")
                        .unwrap()
                        .as_ref()
                        .map(|mo| Oid(mo.parse().unwrap()));
                    let member_of = self
                        .member_ofs
                        .iter_mut()
                        .find(|mo| mo.role_oid == role_oid && mo.member_oid == member_oid)
                        .unwrap();

                    member_of.granted = map.get("granted").unwrap().as_ref().unwrap() == "true";
                    member_of.admin_option =
                        map.get("admin option").unwrap().as_ref().unwrap() == "true";
                    self.member_of_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_members_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.members_dlg.as_mut() {
            match dlg.handle_event(key)? {
                DialogResult::Cancel => self.members_dlg = None,

                DialogResult::Confirm(map) => {
                    let role_oid = map
                        .get("role_oid")
                        .unwrap()
                        .as_ref()
                        .map(|ro| Oid(ro.parse().unwrap()));
                    let member_oid = map
                        .get("member_oid")
                        .unwrap()
                        .as_ref()
                        .map(|mo| Oid(mo.parse().unwrap()));
                    let member = self
                        .members
                        .iter_mut()
                        .find(|mo| mo.role_oid == role_oid && mo.member_oid == member_oid)
                        .unwrap();

                    member.granted = map.get("granted").unwrap().as_ref().unwrap() == "true";
                    member.admin_option =
                        map.get("admin option").unwrap().as_ref().unwrap() == "true";
                    self.members_dlg = None;
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
                        schema: map.get("schema").unwrap().as_ref().unwrap().to_string(),
                        name: map.get("name").unwrap().as_ref().unwrap().to_string(),
                        delete: map.get("delete").unwrap().as_ref().unwrap() == "true",
                        insert: map.get("insert").unwrap().as_ref().unwrap() == "true",
                        references: map.get("references").unwrap().as_ref().unwrap() == "true",
                        select: map.get("select").unwrap().as_ref().unwrap() == "true",
                        trigger: map.get("trigger").unwrap().as_ref().unwrap() == "true",
                        truncate: map.get("truncate").unwrap().as_ref().unwrap() == "true",
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
    async fn handle_input_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.input_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.input_dlg = None;
                }
                DialogResult::Confirm(name) => {
                    let sql = self.build_sql(Some(name.as_str()))?;
                    execute_pg_query_unprepared(
                        self.conns.clone(),
                        self.pools.clone(),
                        &self.conn_id.unwrap(),
                        &sql,
                    )
                    .await?;
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
        if self.role.is_none() {
            self.input_dlg = Some(InputDialog::new("Role Name", None));
        } else {
            let sql = self.build_sql(None)?;
            let sql = sql.trim();
            if !sql.is_empty() {
                execute_pg_query_unprepared(
                    self.conns.clone(),
                    self.pools.clone(),
                    &self.conn_id.unwrap(),
                    sql,
                )
                .await?;
                self.info_dlg = Some(ConfirmDialog::new(
                    ConfirmKind::Info,
                    "Success",
                    "Save Success",
                ));
            }
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_main_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if matches!(*key, BACK_KEY) {
            self.handle_back_event()
        } else if matches!(*key, SAVE_KEY) {
            self.handle_save_event().await
        } else {
            match self.panel {
                PanelKind::General => self.handle_panel_general_event(key),
                PanelKind::MemberOf => self.handle_panel_member_of_event(key),
                PanelKind::Members => self.handle_panel_members_event(key),
                PanelKind::Privileges => self.handle_panel_privileges_event(key).await,
                PanelKind::Comment => self.handle_panel_comment_event(key),
                PanelKind::SQLPreview => self.handle_panel_sql_preview_event(key),
            }
        }
    }
    fn handle_panel_general_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::SQLPreview,
            TAB_RIGHT_KEY => self.panel = PanelKind::MemberOf,

            _ => {
                self.form.handle_event(key)?;
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_panel_member_of_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::General,
            TAB_RIGHT_KEY => self.panel = PanelKind::Members,
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
                    self.member_of_dlg = Some(RoleMemberDialog::new(&self.member_ofs[index]));
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    fn handle_panel_members_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::MemberOf,
            TAB_RIGHT_KEY => self.panel = PanelKind::Privileges,
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
                    self.members_dlg = Some(RoleMemberDialog::new(&self.members[index]));
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_privileges_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::Members,
            TAB_RIGHT_KEY => self.panel = PanelKind::Comment,
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
    fn handle_panel_comment_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::Privileges,
            TAB_RIGHT_KEY => self.panel = PanelKind::SQLPreview,
            _ => {
                let key: Input = key.to_owned().into();
                self.comment.input(key);
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_panel_sql_preview_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::Comment,
            TAB_RIGHT_KEY => self.panel = PanelKind::General,
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    fn clear(&mut self) {
        self.conn_id = None;
        self.role = None;
        self.input_dlg = None;
        self.info_dlg = None;
        self.exit_dlg = None;
        self.panel = PanelKind::General;
        self.member_ofs = Vec::new();
        self.old_member_ofs = Vec::new();
        self.member_ofs_state = TableState::default();
        self.members = Vec::new();
        self.old_members = Vec::new();
        self.members_state = TableState::default();
        self.privileges = Vec::new();
        self.old_privileges = Vec::new();
        self.privileges_state = TableState::default();
        self.delete_privilege_dlg = None;
        self.comment = TextArea::default();
        self.sql_preview = TextArea::default();
        self.form.clear();
        self.member_of_dlg = None;
        self.members_dlg = None;
        self.privilege_dlg = None;
    }
    fn build_sql(&self, role_name: Option<&str>) -> Result<String> {
        if self.role.is_some() {
            self.build_alter_ddl()
        } else {
            Ok(self.build_create_ddl(role_name))
        }
    }
    fn build_alter_ddl(&self) -> Result<String> {
        let mut ddls = Vec::new();

        let role_ddl = self.build_role_ddl()?;
        ddls.extend(role_ddl);
        let member_of_ddl = self.build_member_ofs_ddl();
        ddls.extend(member_of_ddl);
        let members_ddl = self.build_members_ddl();
        ddls.extend(members_ddl);
        let privileges_ddl = self.build_privileges_ddl();
        ddls.extend(privileges_ddl);

        Ok(ddls.join("\n"))
    }
    fn build_role_ddl(&self) -> Result<Vec<String>> {
        let mut ddl = Vec::new();

        let role = self.get_input_role()?;
        let old_role = self.role.as_ref().unwrap();
        if role.super_user() != old_role.super_user() {
            let str = if role.super_user() {
                "SUPERUSER"
            } else {
                "NOSUPERUSER"
            };
            ddl.push(format!("ALTER ROLE \"{}\" WITH {}", role.name(), str));
        }
        if role.inherit() != old_role.inherit() {
            let str = if role.inherit() {
                "INHERIT"
            } else {
                "NOINHERIT"
            };
            ddl.push(format!("ALTER ROLE \"{}\" WITH {}", role.name(), str));
        }
        if role.create_role() != old_role.create_role() {
            let str = if role.create_role() {
                "CREATEROLE"
            } else {
                "NOCREATEROLE"
            };
            ddl.push(format!("ALTER ROLE \"{}\" WITH {}", role.name(), str));
        }
        if role.create_db() != old_role.create_db() {
            let str = if role.create_db() {
                "CREATEDB"
            } else {
                "NOCREATEDB"
            };
            ddl.push(format!("ALTER ROLE \"{}\" WITH {}", role.name(), str));
        }
        if role.can_login() != old_role.can_login() {
            let str = if role.can_login() { "LOGIN" } else { "NOLOGIN" };
            ddl.push(format!("ALTER ROLE \"{}\" WITH {}", role.name(), str));
        }
        if role.replication() != old_role.replication() {
            let str = if role.replication() {
                "REPLICATION"
            } else {
                "NOREPLICATION"
            };
            ddl.push(format!("ALTER ROLE \"{}\" WITH {}", role.name(), str));
        }
        if role.conn_limit() != old_role.conn_limit() {
            ddl.push(format!(
                "ALTER ROLE \"{}\" WITH CONNECTION LIIMIT {}",
                role.name(),
                role.conn_limit()
            ));
        }
        if role.bypassrls() != old_role.bypassrls() {
            let str = if role.bypassrls() {
                "BYPASSRLS"
            } else {
                "NOBYPASSRLS"
            };
            ddl.push(format!("ALTER ROLE \"{}\" WITH {}", role.name(), str));
        }
        if role.expiry_date() != old_role.expiry_date() {
            ddl.push(format!(
                "ALTER ROLE \"{}\" WITH VALID UNTIL '{}'",
                role.name(),
                role.expiry_date()
                    .map(|ed| format!("{}", ed.format("%Y-%m-%d %H:%M:%S")))
                    .unwrap_or("infinity".to_string()),
            ));
        }
        if role.comment() != old_role.comment() {
            ddl.push(format!(
                "COMMENT ON ROLE \"{}\" IS '{}'",
                role.name(),
                role.comment()
            ));
        }

        Ok(ddl)
    }
    fn get_input_role(&self) -> Result<Role> {
        let map = self.form.get_data();
        Ok(Role {
            oid: self.role.as_ref().unwrap().oid,
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            super_user: self.map_str_bool(&map, "superuser"),
            inherit: self.map_str_bool(&map, "inherit privileges"),
            create_role: self.map_str_bool(&map, "can create roles"),
            create_db: self.map_str_bool(&map, "can create databases"),
            can_login: self.map_str_bool(&map, "can login"),
            replication: self.map_str_bool(&map, "can replicate"),
            password: None,
            conn_limit: map
                .get("connect limit")
                .unwrap()
                .as_ref()
                .map(|c| c.parse().unwrap())
                .unwrap_or(-1),
            bypassrls: self.map_str_bool(&map, "can bypass rls"),
            expiry_date: map.get("expiry date").unwrap().as_ref().map(|ed| {
                let ndt = NaiveDateTime::parse_from_str(ed.as_str(), "%Y-%m-%d %H:%M:%S").unwrap();
                Local.from_local_datetime(&ndt).unwrap()
            }),
            comment: self.comment.lines().join("\n"),
        })
    }
    fn build_member_ofs_ddl(&self) -> Vec<String> {
        let mut ddls = Vec::new();
        self.member_ofs.iter().for_each(|mo| {
            let same_mo = self
                .old_member_ofs
                .iter()
                .find(|old_mo| mo.role_oid == old_mo.role_oid && mo.member_oid == old_mo.member_oid)
                .unwrap();
            if let Some(ddl) = mo.get_alter_ddl(same_mo, false) {
                ddls.push(ddl);
            }
        });
        ddls
    }
    fn build_members_ddl(&self) -> Vec<String> {
        let mut ddls = Vec::new();
        self.members.iter().for_each(|mo| {
            let same_mo = self
                .old_members
                .iter()
                .find(|old_mo| mo.role_oid == old_mo.role_oid && mo.member_oid == old_mo.member_oid)
                .unwrap();
            if let Some(ddl) = mo.get_alter_ddl(same_mo, true) {
                ddls.push(ddl);
            }
        });
        ddls
    }
    fn build_privileges_ddl(&self) -> Vec<String> {
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
            .map(|p| p.get_revoke_all_ddl(self.role.as_ref().unwrap().name()))
            .collect();
        ddls.append(&mut revoke_all_ddl);
        self.privileges.iter().for_each(|privilege| {
            if !old_privilege_ids.contains(&privilege.id) {
                let grant_ddl = privilege.get_grant_ddl(self.role.as_ref().unwrap().name());
                ddls.push(grant_ddl);
            } else {
                let same_privilege = self
                    .old_privileges
                    .iter()
                    .find(|p| p.id == privilege.id)
                    .unwrap();
                let mut alter_ddls =
                    privilege.get_alter_ddl(same_privilege, self.role.as_ref().unwrap().name());
                ddls.append(&mut alter_ddls);
            }
        });
        ddls
    }
    fn build_create_ddl(&self, role_name: Option<&str>) -> String {
        let role_name = if let Some(name) = role_name {
            name
        } else {
            "new_role"
        };
        let map = self.form.get_data();
        let in_roles: Vec<String> = self
            .member_ofs
            .iter()
            .filter(|mo| mo.granted)
            .map(|mo| format!("IN ROLE \"{}\"", mo.role_name.as_ref().unwrap()))
            .collect();
        let roles: Vec<String> = self
            .members
            .iter()
            .filter(|ms| ms.granted && !ms.admin_option)
            .map(|ms| format!("ROLE \"{}\"", ms.member_name.as_ref().unwrap()))
            .collect();
        let admin_roles: Vec<String> = self
            .members
            .iter()
            .filter(|ms| ms.granted && ms.admin_option)
            .map(|ms| format!("ADMIN \"{}\"", ms.member_name.as_ref().unwrap()))
            .collect();

        let role_ddl = format!(
            "CREATE ROLE \"{}\" WITH {} {} {} {} {} {} {} {}{}{}{}{}{}",
            role_name,
            self.map_bool_str(&map, "superuser"),
            self.map_bool_str(&map, "can create databases"),
            self.map_bool_str(&map, "can create roles"),
            self.map_bool_str(&map, "inherit priviliges"),
            self.map_bool_str(&map, "can login"),
            self.map_bool_str(&map, "can replicate"),
            self.map_bool_str(&map, "can bypass rls"),
            if let Some(password) = map.get("password") {
                if let Some(password) = password {
                    format!(" PASSWORD '{}'", password)
                } else {
                    String::new()
                }
            } else {
                "PASSWORD NULL".to_string()
            },
            if let Some(expiry) = map.get("expiry date") {
                if let Some(expiry) = expiry {
                    format!(" VALID UNTIL '{}'", expiry)
                } else {
                    String::new()
                }
            } else {
                "".to_string()
            },
            if let Some(connlimit) = map.get("connect limit") {
                if let Some(connlimit) = connlimit {
                    format!(" CONNECTION LIMIT {}", connlimit)
                } else {
                    String::new()
                }
            } else {
                "".to_string()
            },
            if !in_roles.is_empty() {
                in_roles.join(",")
            } else {
                "".to_string()
            },
            if roles.is_empty() {
                roles.join(",")
            } else {
                "".to_string()
            },
            if admin_roles.is_empty() {
                admin_roles.join(",")
            } else {
                "".to_string()
            }
        );
        let privileges_ddl: Vec<String> = self
            .privileges
            .iter()
            .map(|p| {
                let mut p_str = Vec::new();
                if p.select {
                    p_str.push("SELECT");
                }
                if p.delete {
                    p_str.push("DELETE");
                }
                if p.insert {
                    p_str.push("INSERT");
                }
                if p.references {
                    p_str.push("REFERENCES");
                }
                if p.trigger {
                    p_str.push("TRIGGER");
                }
                if p.truncate {
                    p_str.push("TRUNCATE");
                }
                if p.update {
                    p_str.push("UPDATE");
                }
                format!(
                    "GRANT {} ON TABLE \"{}\".\"{}\".\"{}\" TO \"{}\"",
                    p_str.join(","),
                    p.db,
                    p.schema,
                    p.name,
                    role_name,
                )
            })
            .collect();
        let comment = self.comment.lines().join("\n");
        let comment_ddl = if !comment.is_empty() {
            format!("COMMET ON ROLE \"{}\" IS '{}'", role_name, comment)
        } else {
            "".to_string()
        };
        format!(
            "{}\n{}\n{}",
            role_ddl,
            privileges_ddl.join("\n"),
            comment_ddl
        )
    }
    fn map_bool_str(&self, map: &HashMap<String, Option<String>>, key: &str) -> &'static str {
        let val = map.get(key).unwrap().as_ref().unwrap();
        match key {
            "superuser" => {
                if val == "true" {
                    "SUPERUSER"
                } else {
                    "NOSUPERUSER"
                }
            }
            "can create databases" => {
                if val == "true" {
                    "CREATEDB"
                } else {
                    "NOCREATEDB"
                }
            }
            "can create roles" => {
                if val == "true" {
                    "CREATEROLE"
                } else {
                    "NOCREATEROLE"
                }
            }
            "inherit privileges" => {
                if val == "true" {
                    "INHERIT"
                } else {
                    "NOINHERIT"
                }
            }
            "can login" => {
                if val == "true" {
                    "LOGIN"
                } else {
                    "NOLOGIN"
                }
            }
            "can replicate" => {
                if val == "true" {
                    "REPLICATION"
                } else {
                    "NOREPLICATION"
                }
            }
            "can bypass rls" => {
                if val == "true" {
                    "BYPASSRLS"
                } else {
                    "NOBYPASSRLS"
                }
            }

            _ => "",
        }
    }
    fn map_str_bool(&self, map: &HashMap<String, Option<String>>, key: &str) -> bool {
        let val = map.get(key).unwrap().as_ref().unwrap();
        match key {
            "superuser" => val == "true",
            "can create databases" => val == "true",
            "can create roles" => val == "true",
            "inherit privileges" => val == "true",
            "can login" => val == "true",
            "can replicate" => val == "true",
            "can bypass rls" => val == "true",
            _ => false,
        }
    }
    fn update_commands(&self) {
        let mut cmds = if let Some(dlg) = self.member_of_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.members_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.privilege_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.input_dlg.as_ref() {
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
            PanelKind::MemberOf => self.get_panel_member_ofs_commands(),
            PanelKind::Members => self.get_panel_members_commands(),
            PanelKind::Privileges => self.get_panel_privileges_commands(),
            PanelKind::Comment => self.get_panel_comment_commands(),
            PanelKind::SQLPreview => self.get_panel_sql_preview_commands(),
        };
        cmds.extend(vec![
            Command {
                name: "Save Role",
                key: SAVE_KEY,
            },
            Command {
                name: "Back to Roles",
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
                name: "Open Memeber Of",
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
                name: "Open Memebers",
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
    fn get_panel_comment_commands(&self) -> Vec<Command> {
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
