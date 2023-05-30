use crate::{
    app::{ComponentResult, DialogResult, Focus, Goto},
    component::{get_table_down_index, get_table_up_index, Command, CommandBarComponent},
    dialog::{confirm::Kind as ConfirmKind, ConfirmDialog},
    event::{config::*, Key},
    model::pg::{get_pg_roles, Connections, Role},
    pool::{execute_pg_query, get_pg_pool, PGPools},
};
use anyhow::Result;
use std::{cell::RefCell, rc::Rc};
use tui::{
    backend::Backend,
    layout::{Constraint, Margin, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Row as RowUI, Table, TableState},
    Frame,
};
use uuid::Uuid;

pub struct RoleListComponent {
    conn_id: Option<Uuid>,
    state: TableState,
    roles: Vec<Role>,
    delete_dlg: Option<ConfirmDialog>,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<PGPools>>,
    cmd_bar: Rc<RefCell<CommandBarComponent>>,
}

impl RoleListComponent {
    pub fn new(
        cmd_bar: Rc<RefCell<CommandBarComponent>>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<PGPools>>,
    ) -> Self {
        RoleListComponent {
            state: TableState::default(),
            conn_id: None,
            delete_dlg: None,
            roles: Vec::new(),
            conns,
            pools,
            cmd_bar,
        }
    }
    pub async fn set_data(&mut self, conn_id: &Uuid) -> Result<()> {
        self.conn_id = Some(*conn_id);
        let pool = get_pg_pool(
            self.conns.clone(),
            self.pools.clone(),
            self.conn_id.as_ref().unwrap(),
            None,
        )
        .await?;

        self.roles = get_pg_roles(&pool).await?;
        Ok(())
    }
    pub fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect, is_active: bool)
    where
        B: Backend,
    {
        f.render_widget(
            Block::default()
                .title("Roles")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(if is_active {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                }),
            r,
        );

        let table = Table::new(
            self.roles
                .iter()
                .map(|r| {
                    RowUI::new(vec![
                        r.name().to_string(),
                        if r.can_login() { "Y" } else { "N" }.to_string(),
                        r.conn_limit().to_string(),
                        r.expiry_date()
                            .map(|ed| ed.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_default(),
                        r.comment().to_string(),
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec![
            "Name",
            "Can Login",
            "Conn Limit",
            "Expiry Date",
            "Comment",
        ]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
        ])
        .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(
            table,
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
        if let Some(dlg) = self.delete_dlg.as_ref() {
            dlg.draw(f);
        }
    }
    pub async fn handle_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if self.delete_dlg.is_some() {
            self.handle_delete_dlg_event(key).await
        } else {
            self.handle_main_event(key).await
        }
    }
    async fn handle_main_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            UP_KEY => {
                if !self.roles.is_empty() {
                    let index = get_table_up_index(self.state.selected());
                    self.state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.roles.is_empty() {
                    let index = get_table_down_index(self.state.selected(), self.roles.len());
                    self.state.select(Some(index));
                }
            }
            LEFT_KEY => {
                return Ok(ComponentResult::Focus(Focus::LeftPanel));
            }
            NEW_KEY => {
                return Ok(ComponentResult::Goto(Goto::RoleDetailPG {
                    conn_id: self.conn_id.unwrap(),
                    role_name: None,
                }));
            }
            CONFIRM_KEY => {
                if let Some(index) = self.state.selected() {
                    let role = &self.roles[index];
                    return Ok(ComponentResult::Goto(Goto::RoleDetailPG {
                        conn_id: self.conn_id.unwrap(),
                        role_name: Some(role.name().to_string()),
                    }));
                }
            }
            DELETE_KEY => {
                self.delete_dlg = Some(ConfirmDialog::new(
                    ConfirmKind::Warning,
                    "Delete Role",
                    "Are you sure to delete this role?",
                ));
            }
            REFRESH_KEY => {
                self.refresh().await?;
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_delete_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.delete_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.state.selected() {
                        let role = &self.roles[index];
                        execute_pg_query(
                            self.conns.clone(),
                            self.pools.clone(),
                            &self.conn_id.unwrap(),
                            None,
                            &format!("DROP ROLE \"{}\"", role.name()),
                        )
                        .await?;
                        self.delete_dlg = None;
                        self.roles.remove(index);
                        self.state = TableState::default();
                    }
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    pub async fn refresh(&mut self) -> Result<()> {
        let pool = get_pg_pool(
            self.conns.clone(),
            self.pools.clone(),
            self.conn_id.as_ref().unwrap(),
            None,
        )
        .await?;

        self.roles = get_pg_roles(&pool).await?;
        self.state.select(None);
        Ok(())
    }
    fn update_commands(&self) {
        let mut cmds = if let Some(dlg) = self.delete_dlg.as_ref() {
            dlg.get_commands()
        } else {
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
                    name: "New User",
                    key: NEW_KEY,
                },
            ];

            if self.state.selected().is_some() {
                cmds.push(Command {
                    name: "Open Role",
                    key: CONFIRM_KEY,
                });
                cmds.push(Command {
                    name: "Delete Role",
                    key: DELETE_KEY,
                });
            }
            cmds.push(Command {
                name: "Refresh",
                key: REFRESH_KEY,
            });
            cmds.push(Command {
                name: "To Connections",
                key: LEFT_KEY,
            });

            cmds
        };
        self.cmd_bar.borrow_mut().set_commands(&mut cmds);
    }
}
