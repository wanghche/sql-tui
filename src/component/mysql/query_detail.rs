use crate::{
    app::{ComponentResult, DialogResult, MainPanel},
    component::{Command, CommandBarComponent},
    dialog::{DetailDialog, InputDialog},
    event::{config::*, Key},
    model::{
        mysql::{get_mysql_column_value, Connections},
        query::{Queries, Query},
    },
    pool::{fetch_mysql_query, MySQLPools},
};
use anyhow::Result;
use sqlx::{
    mysql::{MySqlColumn, MySqlRow},
    Column as SqlxColumn, Row as SqlxRow,
};
use std::{
    cell::RefCell,
    cmp::{max, min},
    collections::HashMap,
    rc::Rc,
};

use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph, Row as RowUI, Table, TableState},
    Frame,
};
use tui_textarea::{Input, TextArea};
use uuid::Uuid;

enum FocusPanel {
    TextArea,
    Result,
}

pub struct QueryDetailComponent<'a> {
    focus: FocusPanel,
    conn_id: Option<Uuid>,
    db_name: Option<String>,
    query: Option<Query>,
    input: TextArea<'a>,
    rows: Vec<MySqlRow>,
    columns: Vec<MySqlColumn>,
    row_state: TableState,
    is_result: bool,
    detail_dlg: Option<DetailDialog<'a>>,
    input_dlg: Option<InputDialog<'a>>,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<MySQLPools>>,
    queries: Rc<RefCell<Queries>>,
    cmd_bar: Rc<RefCell<CommandBarComponent>>,
}

impl<'a> QueryDetailComponent<'a> {
    pub fn new(
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<MySQLPools>>,
        queries: Rc<RefCell<Queries>>,
        cmd_bar: Rc<RefCell<CommandBarComponent>>,
    ) -> Self {
        QueryDetailComponent {
            focus: FocusPanel::TextArea,
            input: TextArea::default(),
            rows: Vec::new(),
            columns: Vec::new(),
            row_state: TableState::default(),
            is_result: false,
            conn_id: None,
            db_name: None,
            query: None,
            detail_dlg: None,
            input_dlg: None,
            conns,
            pools,
            queries,
            cmd_bar,
        }
    }
    pub fn set_data(
        &mut self,
        conn_id: &Uuid,
        db_name: &str,
        query_name: Option<&str>,
    ) -> Result<()> {
        self.conn_id = Some(*conn_id);
        self.db_name = Some(db_name.to_string());

        if let Some(name) = query_name {
            let mut query = self
                .queries
                .borrow()
                .get_query(conn_id, db_name, name)
                .unwrap();

            self.input = TextArea::from(query.load_file()?.0.split('\n'));
            self.query = Some(query);
        }
        Ok(())
    }
    pub fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect, is_focus: bool)
    where
        B: Backend,
    {
        f.render_widget(
            Block::default()
                .title(if let Some(q) = &self.query {
                    format!(
                        "Query `{}`({})",
                        q.name.as_str(),
                        self.db_name.as_ref().unwrap()
                    )
                } else {
                    format!("New Query({})", self.db_name.as_ref().unwrap())
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
        if self.is_result {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
                .split(r.inner(&Margin {
                    vertical: 1,
                    horizontal: 1,
                }));
            self.draw_query(f, chunks[0]);
            self.draw_result(f, chunks[1]);
        } else {
            self.draw_query(
                f,
                r.inner(&Margin {
                    vertical: 1,
                    horizontal: 1,
                }),
            );
        }
        if is_focus {
            self.update_commands();
        }
    }
    pub fn draw_dialog<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        if let Some(dlg) = self.input_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.detail_dlg.as_mut() {
            dlg.draw(f);
        }
    }
    fn draw_query<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        f.render_widget(self.input.widget(), r);
    }
    fn draw_result<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let block = Block::default()
            .borders(Borders::TOP)
            .title(Span::styled(
                "Result",
                if let FocusPanel::Result = self.focus {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                },
            ))
            .border_style(if let FocusPanel::Result = self.focus {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            });

        if !self.rows.is_empty() {
            let constraints: Vec<Constraint> = self
                .columns
                .iter()
                .map(|_| Constraint::Ratio(1, self.columns.len() as u32))
                .collect();
            let table = Table::new(
                self.rows
                    .iter()
                    .map(|r| {
                        RowUI::new(
                            self.columns
                                .iter()
                                .map(|column| get_mysql_column_value(column, r).unwrap_or_default())
                                .collect::<Vec<String>>(),
                        )
                    })
                    .collect::<Vec<RowUI>>(),
            )
            .header(RowUI::new(
                self.columns.iter().map(|c| c.name()).collect::<Vec<&str>>(),
            ))
            .block(block)
            .widths(&constraints[..])
            .highlight_style(Style::default().fg(Color::Green));

            f.render_stateful_widget(table, r, &mut self.row_state);
        } else {
            f.render_widget(
                Paragraph::new("Success,no data returned.")
                    .block(block)
                    .style(if let FocusPanel::Result = self.focus {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default()
                    }),
                r,
            );
        }
    }
    async fn handle_textarea_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            SWITCH_KEY => {
                if self.is_result {
                    self.focus = FocusPanel::Result;
                    if !self.rows.is_empty() {
                        self.row_state.select(Some(0));
                    }
                }
            }
            _ => {
                let key: Input = key.to_owned().into();
                self.input.input(key);
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_result_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            SWITCH_KEY => {
                self.focus = FocusPanel::TextArea;
                self.row_state.select(None);
            }
            UP_KEY => {
                if !self.rows.is_empty() {
                    self.row_state.select(Some(
                        max(1, self.row_state.selected().unwrap_or_default()) - 1,
                    ));
                }
            }
            DOWN_KEY => {
                if !self.rows.is_empty() {
                    self.row_state.select(Some(min(
                        self.rows.len() - 1,
                        self.row_state.selected().unwrap_or_default() + 1,
                    )));
                }
            }
            CONFIRM_KEY => {
                if let Some(index) = self.row_state.selected() {
                    let mut map = HashMap::new();
                    self.columns.iter().for_each(|col| {
                        map.insert(
                            col.name().to_string(),
                            get_mysql_column_value(col, &self.rows[index]),
                        );
                    });
                    self.detail_dlg = Some(DetailDialog::new("Result".to_string(), &map));
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_main_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if matches!(*key, BACK_KEY) {
            self.clear();
            return Ok(ComponentResult::Back(MainPanel::QueryList));
        } else if matches!(*key, SAVE_KEY) {
            if let Some(query) = self.query.as_ref() {
                let file_path = Queries::get_file_path(query.name())?;
                let mut query = Query::new(
                    self.conn_id.as_ref().unwrap(),
                    self.db_name.as_ref().unwrap(),
                    query.name(),
                    &file_path,
                );
                let query = query.save_file(&self.input.lines().join("\n"))?;
                self.queries.borrow_mut().save_query(query)?;
                self.query = Some(query.clone());
                self.input_dlg = None;
            } else {
                self.input_dlg = Some(InputDialog::new("Query Name", None));
            }
        } else if matches!(*key, RUN_KEY) {
            let sql = self.input.lines().join("\n");
            if !sql.is_empty() {
                self.rows = fetch_mysql_query(
                    self.conns.clone(),
                    self.pools.clone(),
                    self.conn_id.as_ref().unwrap(),
                    self.db_name.as_deref(),
                    &sql,
                )
                .await?;
                self.is_result = true;
                if !self.rows.is_empty() {
                    self.columns = self.rows[0].columns().to_vec();
                    self.row_state.select(Some(0));
                } else {
                    self.columns = vec![];
                    self.row_state.select(None);
                }
            }
        } else {
            match self.focus {
                FocusPanel::TextArea => {
                    self.handle_textarea_event(key).await?;
                }
                FocusPanel::Result => {
                    self.handle_result_event(key)?;
                }
            }
        }
        Ok(ComponentResult::Done)
    }
    pub async fn handle_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if self.detail_dlg.is_some() {
            self.handle_detail_dlg_event(key)
        } else if self.input_dlg.is_some() {
            self.handle_input_dlg_event(key)
        } else {
            self.handle_main_event(key).await
        }
    }
    fn handle_input_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.input_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => self.input_dlg = None,
                DialogResult::Confirm(name) => {
                    let file_path = Queries::get_file_path(&name)?;
                    let mut query = Query::new(
                        self.conn_id.as_ref().unwrap(),
                        self.db_name.as_ref().unwrap(),
                        name.as_str(),
                        &file_path,
                    );
                    let query = query.save_file(&self.input.lines().join("\n"))?;
                    self.queries.borrow_mut().save_query(query)?;
                    self.query = Some(query.clone());
                    self.input_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_detail_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.detail_dlg.as_mut() {
            match dlg.handle_event(key)? {
                DialogResult::Cancel => self.detail_dlg = None,
                DialogResult::Confirm(_) => self.detail_dlg = None,
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn clear(&mut self) {
        self.focus = FocusPanel::TextArea;
        self.conn_id = None;
        self.db_name = None;
        self.query = None;
        self.input = TextArea::default();
        self.rows = Vec::new();
        self.columns = Vec::new();
        self.row_state = TableState::default();
        self.detail_dlg = None;
        self.input_dlg = None;
        self.is_result = false;
    }
    fn update_commands(&self) {
        let mut cmds = if self.input_dlg.is_some() {
            self.input_dlg.as_ref().unwrap().get_commands()
        } else if self.detail_dlg.is_some() {
            self.detail_dlg.as_ref().unwrap().get_commands()
        } else {
            let mut cmds = match self.focus {
                FocusPanel::TextArea => self.get_textarea_commands(),
                FocusPanel::Result => self.get_result_commands(),
            };
            cmds.extend(vec![
                Command {
                    name: "Back",
                    key: BACK_KEY,
                },
                Command {
                    name: "Save",
                    key: SAVE_KEY,
                },
                Command {
                    name: "Run",
                    key: RUN_KEY,
                },
            ]);
            cmds
        };
        self.cmd_bar.borrow_mut().set_commands(&mut cmds);
    }
    fn get_textarea_commands(&self) -> Vec<Command> {
        vec![Command {
            name: "Switch to Result",
            key: SWITCH_KEY,
        }]
    }
    fn get_result_commands(&self) -> Vec<Command> {
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
                name: "Switch to Input",
                key: SWITCH_KEY,
            },
        ];
        if self.row_state.selected().is_some() {
            cmds.push(Command {
                name: "Open Data",
                key: CONFIRM_KEY,
            });
        }
        cmds
    }
}
