use crate::{
    app::{ComponentResult, DialogResult, Focus, Goto},
    component::{get_table_down_index, get_table_up_index, Command, CommandBarComponent},
    dialog::confirm::{ConfirmDialog, Kind as ConfirmKind},
    event::{config::*, Key},
    model::pg::{get_pg_views, Connections, View},
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

pub struct ViewListComponent {
    conn_id: Option<Uuid>,
    db_name: Option<String>,
    schema_name: Option<String>,
    views: Vec<View>,
    state: TableState,
    delete_dlg: Option<ConfirmDialog>,
    cmd_bar: Rc<RefCell<CommandBarComponent>>,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<PGPools>>,
}

impl ViewListComponent {
    pub fn new(
        cmd_bar: Rc<RefCell<CommandBarComponent>>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<PGPools>>,
    ) -> Self {
        ViewListComponent {
            conn_id: None,
            db_name: None,
            schema_name: None,
            views: Vec::new(),
            state: TableState::default(),
            delete_dlg: None,
            cmd_bar,
            conns,
            pools,
        }
    }
    pub async fn set_data(
        &mut self,
        conn_id: &Uuid,
        db_name: &str,
        schema_name: &str,
    ) -> Result<()> {
        self.conn_id = Some(*conn_id);
        self.db_name = Some(db_name.to_string());
        self.schema_name = Some(schema_name.to_string());
        self.state = TableState::default();

        let pool = get_pg_pool(
            self.conns.clone(),
            self.pools.clone(),
            conn_id,
            Some(db_name),
        )
        .await?;
        self.views = get_pg_views(&pool, schema_name).await?;
        Ok(())
    }
    pub fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect, is_focus: bool)
    where
        B: Backend,
    {
        f.render_widget(
            Block::default()
                .title("Views")
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
        let table = Table::new(
            self.views
                .iter()
                .map(|view| RowUI::new(vec![view.name.as_str(), view.comment.as_str()]))
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec!["Name", "Comment"]))
        .block(Block::default())
        .widths(&[Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
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
    }
    pub async fn handle_main_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            UP_KEY => {
                if !self.views.is_empty() {
                    let index = get_table_up_index(self.state.selected());
                    self.state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.views.is_empty() {
                    let index = get_table_down_index(self.state.selected(), self.views.len());
                    self.state.select(Some(index));
                }
            }
            LEFT_KEY => {
                return Ok(ComponentResult::Focus(Focus::LeftPanel));
            }
            CONFIRM_KEY => {
                if let Some(index) = self.state.selected() {
                    return Ok(ComponentResult::Goto(Goto::DataListPG {
                        conn_id: self.conn_id.unwrap(),
                        db_name: self.db_name.clone().unwrap(),
                        schema_name: self.schema_name.clone().unwrap(),
                        table_name: self.views[index].name.clone(),
                    }));
                }
            }
            NEW_KEY => {
                return Ok(ComponentResult::Goto(Goto::ViewDetailPG {
                    conn_id: self.conn_id.unwrap(),
                    db_name: self.db_name.clone().unwrap(),
                    schema_name: self.schema_name.clone().unwrap(),
                    view_name: None,
                }));
            }
            EDIT_KEY => {
                let index = self.state.selected();
                if let Some(i) = index {
                    let view = &self.views[i];
                    return Ok(ComponentResult::Goto(Goto::ViewDetailPG {
                        conn_id: self.conn_id.unwrap(),
                        db_name: self.db_name.clone().unwrap(),
                        schema_name: self.schema_name.clone().unwrap(),
                        view_name: Some(view.name.clone()),
                    }));
                }
            }
            DELETE_KEY => {
                self.delete_dlg = Some(ConfirmDialog::new(
                    ConfirmKind::Warning,
                    "Delete View",
                    "Are you sure to delete this view?",
                ));
            }
            REFRESH_KEY => {
                self.refresh().await?;
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn refresh(&mut self) -> Result<()> {
        let pool = get_pg_pool(
            self.conns.clone(),
            self.pools.clone(),
            self.conn_id.as_ref().unwrap(),
            self.db_name.as_deref(),
        )
        .await?;
        self.views = get_pg_views(&pool, self.db_name.as_ref().unwrap()).await?;

        Ok(())
    }
    async fn handle_delete_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.delete_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.state.selected() {
                        let view = &self.views[index];
                        execute_pg_query(
                            self.conns.clone(),
                            self.pools.clone(),
                            self.conn_id.as_ref().unwrap(),
                            self.db_name.as_deref(),
                            &format!("DROP VIEW \"{}\"", view.name),
                        )
                        .await?;
                        self.delete_dlg = None;
                        self.views.remove(index);
                        self.state = TableState::default();
                    }
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    pub async fn handle_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if self.delete_dlg.is_some() {
            self.handle_delete_dlg_event(key).await
        } else {
            self.handle_main_event(key).await
        }
    }
    pub fn update_commands(&self) {
        let mut cmds = if let Some(dlg) = self.delete_dlg.as_ref() {
            dlg.get_commands()
        } else {
            self.get_main_commands()
        };
        self.cmd_bar.borrow_mut().set_commands(&mut cmds);
    }
    pub fn get_main_commands(&self) -> Vec<Command> {
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
                name: "New View",
                key: NEW_KEY,
            },
        ];
        if self.state.selected().is_some() {
            cmds.append(&mut vec![
                Command {
                    name: "Open View",
                    key: CONFIRM_KEY,
                },
                Command {
                    name: "Edit View",
                    key: EDIT_KEY,
                },
                Command {
                    name: "Delete View",
                    key: DELETE_KEY,
                },
            ]);
        }
        cmds.append(&mut vec![
            Command {
                name: "Refresh",
                key: REFRESH_KEY,
            },
            Command {
                name: "To Connections",
                key: LEFT_KEY,
            },
        ]);
        cmds
    }
}
