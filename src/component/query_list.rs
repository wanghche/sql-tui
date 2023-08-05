use crate::{
    app::{ComponentResult, DialogResult, Focus, Goto},
    component::{get_table_down_index, get_table_up_index, Command, CommandBarComponent},
    dialog::{confirm::Kind as ConfirmKind, ConfirmDialog},
    event::{config::*, Key},
    model::{
        query::{Queries, Query},
        DatabaseKind,
    },
};
use anyhow::Result;
use std::{cell::RefCell, rc::Rc};
use tui::{
    backend::Backend,
    layout::{Constraint, Margin, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Row, Table, TableState},
    Frame,
};
use uuid::Uuid;

pub struct QueryListComponent {
    state: TableState,
    kind: DatabaseKind,
    conn_id: Option<Uuid>,
    db_name: Option<String>,
    query_list: Vec<Query>,
    queries: Rc<RefCell<Queries>>,
    delete_dlg: Option<ConfirmDialog>,
    cmd_bar: Rc<RefCell<CommandBarComponent>>,
}

impl QueryListComponent {
    pub fn new(queries: Rc<RefCell<Queries>>, cmd_bar: Rc<RefCell<CommandBarComponent>>) -> Self {
        QueryListComponent {
            query_list: Vec::new(),
            queries,
            kind: DatabaseKind::MySQL,
            state: TableState::default(),
            delete_dlg: None,
            conn_id: None,
            db_name: None,
            cmd_bar,
        }
    }
    pub fn set_data(&mut self, conn_id: &Uuid, db_name: &str, kind: DatabaseKind) -> Result<()> {
        self.conn_id = Some(*conn_id);
        self.db_name = Some(db_name.to_string());
        self.kind = kind;
        self.state = TableState::default();

        self.query_list = self.queries.borrow().get_queries(conn_id, db_name);
        Ok(())
    }
    pub fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect, is_active: bool)
    where
        B: Backend,
    {
        f.render_widget(
            Block::default()
                .title("Queries")
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

        let data = self
            .query_list
            .iter()
            .map(|q| {
                Row::new(vec![
                    q.name.clone(),
                    q.file_size.to_string(),
                    q.created_date
                        .map(|s| s.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_default(),
                    q.modified_date
                        .map(|s| s.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_default(),
                    q.access_time
                        .map(|s| s.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_default(),
                ])
            })
            .collect::<Vec<Row>>();

        let table = Table::new(data)
            .header(Row::new(vec![
                "Name",
                "File Size",
                "Created Date",
                "Modified Date",
                "Access Time",
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
        if let Some(dlg) = self.delete_dlg.as_mut() {
            dlg.draw(f);
        }
    }

    pub fn handle_main_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            UP_KEY => {
                if !self.query_list.is_empty() {
                    let index = get_table_up_index(self.state.selected());
                    self.state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.query_list.is_empty() {
                    let index = get_table_down_index(self.state.selected(), self.query_list.len());
                    self.state.select(Some(index));
                }
            }
            NEW_KEY => match self.kind {
                DatabaseKind::MySQL => {
                    return Ok(ComponentResult::Goto(Goto::QueryDetailMySQL {
                        conn_id: self.conn_id.unwrap(),
                        db_name: self.db_name.clone().unwrap(),
                        query_name: None,
                    }));
                }
                DatabaseKind::PostgreSQL => {
                    return Ok(ComponentResult::Goto(Goto::QueryDetailPG {
                        conn_id: self.conn_id.unwrap(),
                        db_name: self.db_name.clone().unwrap(),
                        query_name: None,
                    }));
                }
            },
            CONFIRM_KEY => {
                if let Some(index) = self.state.selected() {
                    match self.kind {
                        DatabaseKind::MySQL => {
                            return Ok(ComponentResult::Goto(Goto::QueryDetailMySQL {
                                conn_id: self.conn_id.unwrap(),
                                db_name: self.db_name.clone().unwrap(),
                                query_name: Some(self.query_list[index].name.to_string()),
                            }));
                        }
                        DatabaseKind::PostgreSQL => {
                            return Ok(ComponentResult::Goto(Goto::QueryDetailPG {
                                conn_id: self.conn_id.unwrap(),
                                db_name: self.db_name.clone().unwrap(),
                                query_name: Some(self.query_list[index].name.to_string()),
                            }));
                        }
                    }
                }
            }
            DELETE_KEY => {
                if self.state.selected().is_some() {
                    self.delete_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Warning,
                        "Delete Query",
                        "Are you sure to delete this query?",
                    ));
                }
            }
            LEFT_KEY => {
                return Ok(ComponentResult::Focus(Focus::LeftPanel));
            }
            REFRESH_KEY => {
                self.query_list = self.queries.borrow().get_queries(
                    self.conn_id.as_ref().unwrap(),
                    self.db_name.as_deref().unwrap(),
                );
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    pub fn handle_delete_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.delete_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => self.delete_dlg = None,
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.state.selected() {
                        self.queries
                            .borrow_mut()
                            .delete_query(self.query_list[index].id())?;
                        self.query_list.remove(index);
                        self.delete_dlg = None;
                        self.state.select(None);
                    }
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    pub fn handle_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if self.delete_dlg.is_some() {
            self.handle_delete_dlg_event(key)
        } else {
            self.handle_main_event(key)
        }
    }
    fn update_commands(&mut self) {
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
                name: "New Query",
                key: NEW_KEY,
            },
        ];
        if self.state.selected().is_some() {
            cmds.push(Command {
                name: "Open Query",
                key: CONFIRM_KEY,
            });
            cmds.push(Command {
                name: "Delete Query",
                key: DELETE_KEY,
            });
        }
        cmds.push(Command {
            name: "To Connections",
            key: LEFT_KEY,
        });
        cmds
    }
}
