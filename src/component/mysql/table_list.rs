use crate::{
    app::{ComponentResult, DialogResult, Focus, Goto},
    component::{get_table_down_index, get_table_up_index, Command, CommandBarComponent},
    dialog::{confirm::ConfirmDialog, Kind as ConfirmKind},
    event::{config::*, Key},
    model::mysql::{execute_mysql_table, get_mysql_tables, Connections, Table},
    pool::{get_mysql_pool, MySQLPools},
};
use anyhow::Result;
use std::{cell::RefCell, rc::Rc};
use tui::{
    backend::Backend,
    layout::{Constraint, Margin, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Row as RowUI, Table as TableUI, TableState},
    Frame,
};
use uuid::Uuid;

pub struct TableListComponent {
    conn_id: Option<Uuid>,
    db_name: Option<String>,
    tables: Vec<Table>,
    state: TableState,
    delete_dlg: Option<ConfirmDialog>,
    cmd_bar: Rc<RefCell<CommandBarComponent>>,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<MySQLPools>>,
}

impl TableListComponent {
    pub fn new(
        cmd_bar: Rc<RefCell<CommandBarComponent>>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<MySQLPools>>,
    ) -> Self {
        TableListComponent {
            conn_id: None,
            db_name: None,
            tables: Vec::new(),
            state: TableState::default(),
            delete_dlg: None,
            cmd_bar,
            conns,
            pools,
        }
    }
    pub async fn set_data(&mut self, conn_id: &Uuid, db_name: &str) -> Result<()> {
        self.conn_id = Some(*conn_id);
        self.db_name = Some(db_name.to_string());
        self.state = TableState::default();

        let pool = get_mysql_pool(
            self.conns.clone(),
            self.pools.clone(),
            conn_id,
            Some("information_schema"),
        )
        .await?;
        self.tables = get_mysql_tables(&pool, db_name).await?;
        Ok(())
    }
    pub fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect, is_focus: bool)
    where
        B: Backend,
    {
        f.render_widget(
            Block::default()
                .title("Tables")
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
        let table = TableUI::new(
            self.tables
                .iter()
                .map(|t| {
                    RowUI::new(vec![
                        t.name.clone(),
                        t.rows.map(|r| r.to_string()).unwrap_or_default(),
                        t.data_length.map(|dl| dl.to_string()).unwrap_or_default(),
                        t.engine.clone().unwrap_or_default(),
                        t.create_date
                            .map(|m| m.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_default(),
                        t.modified_date
                            .map(|m| m.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_default(),
                        t.collation.clone().unwrap_or_default(),
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec![
            "name",
            "rows",
            "data length",
            "engine",
            "create date",
            "modified date",
            "collation",
        ]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 7),
            Constraint::Ratio(1, 7),
            Constraint::Ratio(1, 7),
            Constraint::Ratio(1, 7),
            Constraint::Ratio(1, 7),
            Constraint::Ratio(1, 7),
            Constraint::Ratio(1, 7),
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
    pub async fn handle_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if self.delete_dlg.is_some() {
            self.handle_delete_dlg_event(key).await
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
                        let table = &self.tables[index];

                        execute_mysql_table(
                            self.conns.clone(),
                            self.pools.clone(),
                            &self.conn_id.unwrap(),
                            self.db_name.as_ref().unwrap(),
                            &format!("DROP TABLE `{}`", &table.name),
                        )
                        .await?;
                        self.tables.remove(index);
                        self.delete_dlg = None;
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
                if !self.tables.is_empty() {
                    let index = get_table_up_index(self.state.selected());
                    self.state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.tables.is_empty() {
                    let index = get_table_down_index(self.state.selected(), self.tables.len());
                    self.state.select(Some(index));
                }
            }
            LEFT_KEY => {
                return Ok(ComponentResult::Focus(Focus::LeftPanel));
            }
            CONFIRM_KEY => {
                if let Some(index) = self.state.selected() {
                    let table = &self.tables[index];
                    return Ok(ComponentResult::Goto(Goto::DataListMySQL {
                        conn_id: self.conn_id.unwrap(),
                        db_name: self.db_name.clone().unwrap(),
                        table_name: table.name.clone(),
                    }));
                }
            }
            NEW_KEY => {
                return Ok(ComponentResult::Goto(Goto::TableDetailMySQL {
                    conn_id: self.conn_id.unwrap(),
                    db_name: self.db_name.clone().unwrap(),
                    table_name: None,
                }));
            }
            EDIT_KEY => {
                if let Some(index) = self.state.selected() {
                    return Ok(ComponentResult::Goto(Goto::TableDetailMySQL {
                        conn_id: self.conn_id.unwrap(),
                        db_name: self.db_name.clone().unwrap(),
                        table_name: Some(self.tables[index].name.clone()),
                    }));
                }
            }
            DELETE_KEY => {
                self.delete_dlg = Some(ConfirmDialog::new(
                    ConfirmKind::Warning,
                    "Delete Table",
                    "Are you sure to delete this table?",
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
        let pool = get_mysql_pool(
            self.conns.clone(),
            self.pools.clone(),
            self.conn_id.as_ref().unwrap(),
            Some("information_schema"),
        )
        .await?;
        self.tables = get_mysql_tables(&pool, self.db_name.as_ref().unwrap()).await?;
        Ok(())
    }
    fn update_commands(&self) {
        let mut cmds = if let Some(dlg) = self.delete_dlg.as_ref() {
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
                name: "New Table",
                key: NEW_KEY,
            },
        ];
        if self.state.selected().is_some() {
            cmds.append(&mut vec![
                Command {
                    name: "Open Table",
                    key: CONFIRM_KEY,
                },
                Command {
                    name: "Edit Table",
                    key: EDIT_KEY,
                },
                Command {
                    name: "Delete Table",
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
