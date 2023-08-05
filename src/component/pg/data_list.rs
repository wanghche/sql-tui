use crate::{
    app::{ComponentResult, DialogResult, MainPanel},
    component::{get_table_down_index, get_table_up_index, Command, CommandBarComponent},
    dialog::DetailDialog,
    event::{config::*, Key},
    model::pg::{convert_show_column_to_pg_fields, get_pg_field_value, Connections, Field},
    pool::{fetch_one_pg, fetch_pg_query, PGPools},
};
use anyhow::Result;
use sqlx::{postgres::PgRow, Row};
use std::{cell::RefCell, rc::Rc};
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
    detail_dlg: Option<DetailDialog<'a>>,
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
            detail_dlg: None,
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
        self.rows = fetch_pg_query(
            self.conns.clone(),
            self.pools.clone(),
            self.conn_id.as_ref().unwrap(),
            Some(db_name),
            &format!(
                "SELECT * FROM {} LIMIT 0 OFFSET {}",
                table_name, self.page_size,
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
        .unwrap()
        .try_get(0)
        .unwrap();

        self.page = if total_count > 0 { 1 } else { 0 };
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
        if let Some(dlg) = self.detail_dlg.as_mut() {
            dlg.draw(f);
        }
    }
    pub async fn handle_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if self.detail_dlg.is_some() {
            self.handle_detail_dlg_event(key).await
        } else {
            self.handle_main_event(key).await
        }
    }
    async fn handle_detail_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.detail_dlg.as_mut() {
            match dlg.handle_event(key)? {
                DialogResult::Cancel => {
                    self.detail_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    self.detail_dlg = None;
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
            CONFIRM_KEY => {
                if let Some(index) = self.state.selected() {
                    let detail_dlg = DetailDialog::from_pg_row(
                        self.table_name.as_ref().unwrap().to_string(),
                        &self.fields,
                        &self.rows[index],
                    );
                    self.detail_dlg = Some(detail_dlg);
                }
            }
            REFRESH_KEY => {
                self.refresh().await?;
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
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
        .unwrap()
        .try_get(0)
        .unwrap();
        self.total_page = (total_count as f64 / self.page_size as f64).ceil() as usize;
        Ok(())
    }
    fn update_commands(&mut self) {
        let mut cmds = if let Some(dlg) = self.detail_dlg.as_mut() {
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
                name: "Page Prev",
                key: PAGE_PRIV_KEY,
            },
        ];
        if self.state.selected().is_some() {
            cmds.append(&mut vec![Command {
                name: "Open",
                key: CONFIRM_KEY,
            }]);
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
