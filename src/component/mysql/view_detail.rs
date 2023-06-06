use crate::{
    app::{ComponentResult, DialogResult, MainPanel},
    component::{Command, CommandBarComponent},
    dialog::{confirm::Kind as ConfirmKind, ConfirmDialog, InputDialog},
    event::{config::*, Key},
    model::mysql::Connections,
    pool::{execute_mysql_query_unprepared, fetch_one_mysql, MySQLPools},
    widget::{Form, FormItem},
};
use anyhow::Result;
use regex::Regex;
use sqlx::Row;
use std::{cell::RefCell, rc::Rc};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, Tabs},
    Frame,
};
use tui_textarea::{Input, TextArea};
use uuid::Uuid;

pub enum PanelKind {
    Definition,
    Advanced,
    SQLPreview,
}

pub struct ViewDetailComponent<'a> {
    conn_id: Option<Uuid>,
    db_name: Option<String>,
    view_name: Option<String>,
    panel: PanelKind,
    input_dlg: Option<InputDialog<'a>>,
    definition: TextArea<'a>,
    old_definition: TextArea<'a>,
    form: Form<'a>,
    old_form: Form<'a>,
    sql_preview: TextArea<'a>,
    exit_dlg: Option<ConfirmDialog>,
    info_dlg: Option<ConfirmDialog>,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<MySQLPools>>,
    cmd_bar: Rc<RefCell<CommandBarComponent>>,
}

impl<'a> ViewDetailComponent<'a> {
    pub fn new(
        cmd_bar: Rc<RefCell<CommandBarComponent>>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<MySQLPools>>,
    ) -> Self {
        ViewDetailComponent {
            conn_id: None,
            db_name: None,
            view_name: None,
            input_dlg: None,
            exit_dlg: None,
            info_dlg: None,
            panel: PanelKind::Definition,
            definition: TextArea::default(),
            old_definition: TextArea::default(),
            sql_preview: TextArea::default(),
            form: Form::default(),
            old_form: Form::default(),
            conns,
            pools,
            cmd_bar,
        }
    }
    pub async fn set_data(
        &mut self,
        conn_id: &Uuid,
        db_name: &str,
        view_name: Option<&str>,
    ) -> Result<()> {
        self.conn_id = Some(*conn_id);
        self.db_name = Some(db_name.to_string());
        self.view_name = view_name.map(|s| s.to_string());
        if let Some(name) = view_name {
            let create_view = fetch_one_mysql(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                self.db_name.as_deref(),
                &format!("SHOW CREATE VIEW `{}`", name),
            )
            .await?;
            let view = fetch_one_mysql(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some("information_schema"),
                &format!(
                    "SELECT * FROM VIEWS WHERE TABLE_SCHEMA='{}' AND TABLE_NAME='{}'",
                    self.db_name.as_ref().unwrap(),
                    name
                ),
            )
            .await?;
            let reg = Regex::new(r"^CREATE\s(ALGORITHM=(?P<algorithm>MERGE|UNDEFINED|TEMPTABLE))?")
                .unwrap();
            let caps = reg.captures(create_view.try_get(1).unwrap()).unwrap();
            self.definition = TextArea::from([view
                .try_get::<String, _>("VIEW_DEFINITION")
                .unwrap()
                .as_str()]);
            self.form.set_items(vec![
                FormItem::new_select(
                    "Algorithm".to_string(),
                    vec![
                        "UNDEFINED".to_string(),
                        "MERGE".to_string(),
                        "TEMPTABLE".to_string(),
                    ],
                    caps.name("algorithm").map(|a| a.as_str().to_string()),
                    true,
                    false,
                ),
                FormItem::new_input(
                    "Definer".to_string(),
                    Some(view.try_get::<String, _>("DEFINER").unwrap().as_str()),
                    true,
                    false,
                    false,
                ),
                FormItem::new_select(
                    "Check Option".to_string(),
                    vec![
                        "NONE".to_string(),
                        "CASCADED".to_string(),
                        "LOCAL".to_string(),
                    ],
                    Some(view.try_get::<String, _>("CHECK_OPTION").unwrap()),
                    true,
                    false,
                ),
                FormItem::new_select(
                    "Security".to_string(),
                    vec!["DEFINER".to_string(), "INVOKER".to_string()],
                    Some(view.try_get::<String, _>("SECURITY_TYPE").unwrap()),
                    true,
                    false,
                ),
            ]);
        } else {
            self.definition = TextArea::default();
            self.form.set_items(vec![
                FormItem::new_select(
                    "Algorithm".to_string(),
                    vec![
                        "UNDEFINED".to_string(),
                        "MERGE".to_string(),
                        "TEMPTABLE".to_string(),
                    ],
                    None,
                    true,
                    false,
                ),
                FormItem::new_input("Definer".to_string(), None, true,false, false),
                FormItem::new_select(
                    "Check Option".to_string(),
                    vec![
                        "NONE".to_string(),
                        "CASCADED".to_string(),
                        "LOCAL".to_string(),
                    ],
                    None,
                    true,
                    false,
                ),
                FormItem::new_select(
                    "Security".to_string(),
                    vec!["DEFINER".to_string(), "INVOKER".to_string()],
                    None,
                    true,
                    false,
                ),
            ]);
        }
        self.old_form = self.form.clone();
        self.old_definition = self.definition.clone();
        Ok(())
    }

    pub fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect, is_focus: bool)
    where
        B: Backend,
    {
        f.render_widget(
            Block::default()
                .title(if let Some(name) = self.view_name.as_ref() {
                    format!("Edit View `{}`", name)
                } else {
                    "New View".to_string()
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
        let selected_tab = match self.panel {
            PanelKind::Definition => 0,
            PanelKind::Advanced => 1,
            PanelKind::SQLPreview => 2,
        };
        f.render_widget(
            Tabs::new(
                [
                    Span::raw("Definition"),
                    Span::raw("Advanced"),
                    Span::raw("SQL Preview"),
                ]
                .iter()
                .cloned()
                .map(Spans::from)
                .collect(),
            )
            .block(Block::default().borders(Borders::BOTTOM))
            .highlight_style(Style::default().fg(Color::Green))
            .select(selected_tab),
            chunks[0],
        );
        match self.panel {
            PanelKind::Definition => self.draw_definition(f, chunks[1]),
            PanelKind::Advanced => self.draw_advanced(f, chunks[1]),
            PanelKind::SQLPreview => self.draw_sql_preview(f, chunks[1]),
        }
        if is_focus {
            self.update_commands();
        }
    }
    pub fn draw_dialog<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        if let Some(dlg) = self.input_dlg.as_ref() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.exit_dlg.as_ref() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.info_dlg.as_ref() {
            dlg.draw(f);
        }
    }
    fn draw_definition<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        f.render_widget(self.definition.widget(), r);
    }
    fn draw_advanced<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(r);
        self.form.draw(f, chunks[0]);
    }
    fn draw_sql_preview<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let sql = self.build_sql(None);
        self.sql_preview = TextArea::from(sql.lines());
        f.render_widget(self.sql_preview.widget(), r);
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
        if self.view_name.is_some() {
            let sql = self.build_sql(None);
            let sql = sql.trim();
            if !sql.is_empty() {
                execute_mysql_query_unprepared(
                    self.conns.clone(),
                    self.pools.clone(),
                    self.conn_id.as_ref().unwrap(),
                    self.db_name.as_deref(),
                    sql,
                )
                .await?;
                self.info_dlg = Some(ConfirmDialog::new(
                    ConfirmKind::Info,
                    "Success",
                    "Save Success",
                ));
            }
        } else {
            self.input_dlg = Some(InputDialog::new("View Name", None));
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
                PanelKind::Definition => self.handle_panel_definition_event(key),
                PanelKind::Advanced => self.handle_panel_advanced_event(key),
                PanelKind::SQLPreview => self.handle_panel_sql_preview_event(key),
            }
        }
    }
    fn handle_panel_definition_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_RIGHT_KEY => self.panel = PanelKind::Advanced,
            TAB_LEFT_KEY => self.panel = PanelKind::SQLPreview,
            _ => {
                let input: Input = key.to_owned().into();
                self.definition.input(input);
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_panel_advanced_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_RIGHT_KEY => self.panel = PanelKind::SQLPreview,
            TAB_LEFT_KEY => self.panel = PanelKind::Definition,
            _ => {
                self.form.handle_event(key)?;
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_panel_sql_preview_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::Advanced,
            TAB_RIGHT_KEY => self.panel = PanelKind::Definition,
            _ => (),
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
                    let sql = self.build_sql(Some(name.as_str()));
                    let sql = sql.trim();
                    if !sql.is_empty() {
                        execute_mysql_query_unprepared(
                            self.conns.clone(),
                            self.pools.clone(),
                            self.conn_id.as_ref().unwrap(),
                            self.db_name.as_deref(),
                            sql,
                        )
                        .await?;
                        self.input_dlg = None;
                        self.view_name = Some(name.to_string());
                        self.info_dlg = Some(ConfirmDialog::new(
                            ConfirmKind::Info,
                            "Success",
                            "Save Success",
                        ));
                    }
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_exit_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.exit_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => self.exit_dlg = None,
                DialogResult::Confirm(_) => {
                    self.clear();
                    return Ok(ComponentResult::Back(MainPanel::ViewListMySQL));
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_info_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.info_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => self.info_dlg = None,
                DialogResult::Confirm(_) => {
                    self.old_definition = self.definition.clone();
                    self.old_form = self.form.clone();
                    self.info_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    pub async fn handle_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if self.input_dlg.is_some() {
            self.handle_input_dlg_event(key).await
        } else if self.exit_dlg.is_some() {
            self.handle_exit_dlg_event(key)
        } else if self.info_dlg.is_some() {
            self.handle_info_dlg_event(key)
        } else {
            self.handle_main_event(key).await
        }
    }
    fn clear(&mut self) {
        self.conn_id = None;
        self.db_name = None;
        self.view_name = None;
        self.panel = PanelKind::Definition;
        self.input_dlg = None;
        self.definition = TextArea::default();
        self.old_definition = TextArea::default();
        self.form.clear();
        self.old_form.clear();
        self.sql_preview = TextArea::default();
        self.exit_dlg = None;
    }
    fn build_create_ddl(&self, view_name: Option<&str>) -> String {
        let name = if let Some(name) = view_name {
            name
        } else {
            "new view"
        };
        let map = self.form.get_data();
        let algorithm = if let Some(algorithm) = map.get("Algorithm") {
            if let Some(algorithm) = algorithm {
                format!(" ALGORITHM={}", algorithm)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let definer = if let Some(definer) = map.get("Definer").unwrap() {
            if !definer.is_empty() {
                format!(" DEFINER = {}", definer)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let security = if let Some(security) = map.get("Security") {
            if let Some(security) = security {
                format!(" SQL SECURITY {}", security)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let check_option = if let Some(check) = map.get("Check Option") {
            if let Some(check) = check {
                format!(" WITH {} CHECK OPTION", check)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        format!(
            "CREATE{}{}{} VIEW `{}` AS {}{}",
            algorithm,
            definer,
            security,
            name,
            self.definition.lines().join("\n"),
            check_option
        )
    }
    fn build_alter_ddl(&self) -> String {
        let map = self.form.get_data();
        let old_map = self.old_form.get_data();

        let algorithm = if map.get("Algorithm") != old_map.get("Algorithm") {
            if let Some(algorithm) = map.get("Algorithm") {
                if let Some(algorithm) = algorithm {
                    format!(" ALGORITHM={}", algorithm)
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let definer = if map.get("Definer") != old_map.get("Definer") {
            if let Some(definer) = map.get("Definer") {
                if let Some(definer) = definer {
                    format!(" DEFINER={}", definer)
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let security = if map.get("Security") != old_map.get("Security") {
            if let Some(security) = map.get("Security") {
                if let Some(security) = security {
                    format!(" SQL SECURITY {}", security)
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let check_option = if map.get("Check Option") != old_map.get("Check Option") {
            if let Some(check_option) = map.get("Check Option") {
                if let Some(check_option) = check_option {
                    format!(" WITH {} CHECK OPTION", check_option)
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        format!(
            "ALTER{}{}{} VIEW `{}` AS {}{}",
            algorithm,
            definer,
            security,
            self.view_name.as_ref().unwrap(),
            self.definition.lines().join("\n"),
            check_option
        )
    }
    fn build_sql(&self, view_name: Option<&str>) -> String {
        if self.view_name.is_some() {
            self.build_alter_ddl()
        } else {
            self.build_create_ddl(view_name)
        }
    }
    fn update_commands(&mut self) {
        let mut cmds = if let Some(dlg) = self.input_dlg.as_mut() {
            dlg.get_commands()
        } else if let Some(dlg) = self.exit_dlg.as_mut() {
            dlg.get_commands()
        } else {
            self.get_main_commands()
        };

        self.cmd_bar.borrow_mut().set_commands(&mut cmds);
    }
    fn get_main_commands(&self) -> Vec<Command> {
        let mut cmds = match self.panel {
            PanelKind::Definition => self.get_panel_definition_commands(),
            PanelKind::Advanced => self.get_panel_advanced_commands(),
            PanelKind::SQLPreview => self.get_panel_sql_preview_commands(),
        };
        cmds.extend(vec![
            Command {
                name: "Save View",
                key: SAVE_KEY,
            },
            Command {
                name: "Back to Views",
                key: BACK_KEY,
            },
        ]);
        cmds
    }
    fn get_panel_definition_commands(&self) -> Vec<Command> {
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
    fn get_panel_advanced_commands(&self) -> Vec<Command> {
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
