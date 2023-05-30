use crate::{
    app::{ComponentResult, DialogResult, MainPanel},
    component::{get_table_down_index, get_table_up_index, Command, CommandBarComponent},
    dialog::{confirm::Kind as ConfirmKind, pg::RuleDialog, ConfirmDialog, InputDialog},
    event::{config::*, Key},
    model::pg::{get_pg_role_names, get_pg_view, Connections, Rule, View},
    pool::{execute_pg_query, execute_pg_query_unprepared, get_pg_pool, PGPools},
    widget::{Form, FormItem},
};
use anyhow::Result;
use std::{cell::RefCell, rc::Rc};
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

pub enum PanelKind {
    Definition,
    Rules,
    Advanced,
    Comment,
    SQLPreview,
}

pub struct ViewDetailComponent<'a> {
    conn_id: Option<Uuid>,
    db_name: Option<String>,
    schema_name: Option<String>,
    panel: PanelKind,
    view: Option<View>,
    input_dlg: Option<InputDialog<'a>>,
    exit_dlg: Option<ConfirmDialog>,
    delete_rule_dlg: Option<ConfirmDialog>,
    info_dlg: Option<ConfirmDialog>,
    definition: TextArea<'a>,
    comment: TextArea<'a>,
    sql_preview: TextArea<'a>,
    form: Form<'a>,
    rules: Vec<Rule>,
    rule_dlg: Option<RuleDialog<'a>>,
    rules_state: TableState,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<PGPools>>,
    cmd_bar: Rc<RefCell<CommandBarComponent>>,
}

impl<'a> ViewDetailComponent<'a> {
    pub fn new(
        cmd_bar: Rc<RefCell<CommandBarComponent>>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<PGPools>>,
    ) -> Self {
        let mut form = Form::default();
        form.set_items(vec![FormItem::new_select(
            "Owner".to_string(),
            vec![],
            None,
            true,
            false,
        )]);
        ViewDetailComponent {
            conn_id: None,
            db_name: None,
            schema_name: None,
            view: None,
            input_dlg: None,
            rule_dlg: None,
            delete_rule_dlg: None,
            info_dlg: None,
            rules_state: TableState::default(),
            exit_dlg: None,
            panel: PanelKind::Definition,
            definition: TextArea::default(),
            comment: TextArea::default(),
            sql_preview: TextArea::default(),
            rules: Vec::new(),
            form,
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
        view_name: Option<&str>,
    ) -> Result<()> {
        self.conn_id = Some(*conn_id);
        self.db_name = Some(db_name.to_string());
        self.schema_name = Some(schema_name.to_string());

        let pool = get_pg_pool(
            self.conns.clone(),
            self.pools.clone(),
            self.conn_id.as_ref().unwrap(),
            self.db_name.as_deref(),
        )
        .await?;
        if let FormItem::Select { options, .. } = self.form.get_item_mut("Owner").unwrap() {
            *options = get_pg_role_names(&pool).await?;
        }

        if let Some(name) = view_name {
            let view = get_pg_view(&pool, self.schema_name.as_deref().unwrap(), name).await?;

            self.definition.insert_str(view.definition.clone());
            if !view.comment.is_empty() {
                self.comment.insert_str(view.comment.clone());
            }
            self.rules = view.rules.clone();

            if let FormItem::Select { selected, .. } = self.form.get_item_mut("Owner").unwrap() {
                *selected = view.owner.clone();
            }

            self.view = Some(view);
        }
        Ok(())
    }

    pub fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect, is_focus: bool)
    where
        B: Backend,
    {
        f.render_widget(
            Block::default()
                .title(if let Some(view) = &self.view {
                    format!("Edit View `{}`", view.name)
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
            PanelKind::Rules => 1,
            PanelKind::Advanced => 2,
            PanelKind::Comment => 3,
            PanelKind::SQLPreview => 4,
        };
        f.render_widget(
            Tabs::new(
                [
                    Span::raw("Definition"),
                    Span::raw("Rules"),
                    Span::raw("Advanced"),
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
            .select(selected_tab),
            chunks[0],
        );
        match self.panel {
            PanelKind::Definition => self.draw_definition(f, chunks[1]),
            PanelKind::Rules => self.draw_rules(f, chunks[1]),
            PanelKind::Advanced => self.draw_advanced(f, chunks[1]),
            PanelKind::Comment => self.draw_comment(f, chunks[1]),
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
        if let Some(dlg) = self.delete_rule_dlg.as_ref() {
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
    fn draw_rules<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(
            self.rules
                .iter()
                .map(|r| {
                    RowUI::new(vec![
                        r.name(),
                        r.event(),
                        r.do_instead().unwrap_or(""),
                        r.where_condition().unwrap_or(""),
                        r.definition().unwrap_or(""),
                        r.comment().unwrap_or(""),
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec![
            "Name",
            "Event",
            "Do Instead",
            "Where",
            "Definition",
            "Comment",
        ]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
            Constraint::Ratio(1, 6),
        ])
        .highlight_style(Style::default().fg(Color::Green));

        f.render_stateful_widget(table, r, &mut self.rules_state);
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
    fn draw_comment<B>(&self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        f.render_widget(self.comment.widget(), r);
    }
    fn draw_sql_preview<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let sql = self.build_sql(None);
        self.sql_preview = TextArea::from(sql.lines());
        f.render_widget(self.sql_preview.widget(), r);
    }
    async fn handle_main_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if matches!(*key, SAVE_KEY) {
            if self.view.is_none() {
                self.input_dlg = Some(InputDialog::new("View Name", None));
            } else {
                let sql = self.build_sql(None);
                let sql = sql.trim();
                if !sql.is_empty() {
                    execute_pg_query(
                        self.conns.clone(),
                        self.pools.clone(),
                        self.conn_id.as_ref().unwrap(),
                        self.db_name.as_deref(),
                        &self.build_sql(None),
                    )
                    .await?;
                    let mut view = self.view.as_mut().unwrap();
                    view.definition = self.definition.lines().join("\n");
                    view.rules = self.rules.clone();
                    view.owner = self.form.get_value("Owner");
                    view.comment = self.comment.lines().join("\n");
                    self.info_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Info,
                        "Success",
                        "Save Success",
                    ));
                }
            }
            Ok(ComponentResult::Done)
        } else if matches!(*key, BACK_KEY) {
            self.exit_dlg = Some(ConfirmDialog::new(
                ConfirmKind::Confirm,
                "Exit",
                "Are you sure to exit?",
            ));
            Ok(ComponentResult::Done)
        } else {
            match self.panel {
                PanelKind::Definition => self.handle_panel_definition_event(key),
                PanelKind::Rules => self.handle_panel_rules_event(key),
                PanelKind::Advanced => self.handle_panel_advanced_event(key),
                PanelKind::Comment => self.handle_panel_comment_event(key),
                PanelKind::SQLPreview => self.handle_panel_sql_preview_event(key),
            }
        }
    }
    fn handle_panel_definition_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::SQLPreview,
            TAB_RIGHT_KEY => self.panel = PanelKind::Rules,
            _ => {
                let key: Input = key.to_owned().into();
                self.definition.input(key);
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_panel_advanced_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::Rules,
            TAB_RIGHT_KEY => self.panel = PanelKind::Comment,
            _ => {
                self.form.handle_event(key)?;
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_panel_rules_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            NEW_KEY => self.rule_dlg = Some(RuleDialog::new(None)),
            CONFIRM_KEY => {
                if let Some(index) = self.rules_state.selected() {
                    self.rule_dlg = Some(RuleDialog::new(Some(&self.rules[index])));
                }
            }
            DELETE_KEY => {
                if self.rules_state.selected().is_some() {
                    self.delete_rule_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Warning,
                        "Delete Rule",
                        "Are you sure to delete this rule?",
                    ));
                }
            }
            TAB_LEFT_KEY => {
                self.panel = PanelKind::Definition;
            }
            TAB_RIGHT_KEY => {
                self.panel = PanelKind::Advanced;
            }
            UP_KEY => {
                if !self.rules.is_empty() {
                    let index = get_table_up_index(self.rules_state.selected());
                    self.rules_state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.rules.is_empty() {
                    let index = get_table_down_index(self.rules_state.selected(), self.rules.len());
                    self.rules_state.select(Some(index));
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_input_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.input_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => self.input_dlg = None,
                DialogResult::Confirm(name) => {
                    let sql = self.build_sql(Some(name.as_str()));
                    let sql = sql.trim();
                    if !sql.is_empty() {
                        execute_pg_query_unprepared(
                            self.conns.clone(),
                            self.pools.clone(),
                            &self.conn_id.unwrap(),
                            sql,
                        )
                        .await?;
                        self.input_dlg = None;
                    }
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_panel_comment_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::Advanced,
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
            TAB_RIGHT_KEY => self.panel = PanelKind::Definition,
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    fn handle_exit_dlg_event(&mut self, key: &Key) -> ComponentResult {
        if let Some(dlg) = self.exit_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.exit_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    self.clear();
                    return ComponentResult::Back(MainPanel::ViewListPG);
                }
                _ => (),
            }
        }
        ComponentResult::Done
    }
    fn handle_delete_rule_dlg_event(&mut self, key: &Key) -> ComponentResult {
        if let Some(dlg) = self.delete_rule_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_rule_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.rules_state.selected() {
                        self.rules.remove(index);
                        self.rules_state.select(None);
                        self.delete_rule_dlg = None;
                    }
                }
                _ => (),
            }
        }
        ComponentResult::Done
    }
    fn handle_info_dlg_event(&mut self, key: &Key) -> ComponentResult {
        if let Some(dlg) = self.info_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.info_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    self.info_dlg = None;
                }
                _ => (),
            }
        }
        ComponentResult::Done
    }
    pub async fn handle_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if self.input_dlg.is_some() {
            self.handle_input_dlg_event(key).await
        } else if self.exit_dlg.is_some() {
            Ok(self.handle_exit_dlg_event(key))
        } else if self.delete_rule_dlg.is_some() {
            Ok(self.handle_delete_rule_dlg_event(key))
        } else if self.info_dlg.is_some() {
            Ok(self.handle_info_dlg_event(key))
        } else {
            self.handle_main_event(key).await
        }
    }
    fn clear(&mut self) {
        self.conn_id = None;
        self.db_name = None;
        self.schema_name = None;
        self.view = None;
        self.panel = PanelKind::Definition;
        self.input_dlg = None;
        self.exit_dlg = None;
        self.delete_rule_dlg = None;
        self.definition = TextArea::default();
        self.comment = TextArea::default();
        self.sql_preview = TextArea::default();
        self.rules = Vec::new();
        self.rule_dlg = None;
        self.rules_state = TableState::default();
        self.form.clear();
    }
    fn build_create_ddl(&self, view_name: Option<&str>) -> String {
        let view_name = if let Some(name) = view_name {
            name
        } else {
            "new view"
        };

        let view_ddl = format!(
            "CREATE VIEW \"{}\" AS {}",
            view_name,
            self.definition.lines().join("\n")
        );

        let owner_option = self.form.get_value("Owner");

        let alter_view_ddl = if let Some(owner) = owner_option {
            format!("ALTER VIEW \"{}\" OWNER TO {}", view_name, owner)
        } else {
            String::new()
        };
        let rules_ddl = if !self.rules.is_empty() {
            self.rules
                .iter()
                .map(|rule| rule.get_create_ddl(self.schema_name.as_deref().unwrap(), view_name))
                .collect::<Vec<String>>()
                .join("\n")
        } else {
            String::new()
        };
        let comment = self.comment.lines().join("\n");
        let comment_ddl = if !comment.is_empty() {
            format!("COMMENT ON {}", comment)
        } else {
            String::new()
        };
        format!(
            "{}\n{}\n{}\n{}",
            view_ddl, alter_view_ddl, rules_ddl, comment_ddl
        )
    }
    fn build_alter_ddl(&self) -> String {
        let mut ddl = Vec::new();
        if let Some(view) = self.view.as_ref() {
            if view.definition != self.definition.lines().join("\n") {
                ddl.push(format!(
                    "DROP VIEW \"{}\".\"{}\"",
                    self.schema_name.as_deref().unwrap(),
                    view.name
                ));
                ddl.push(self.build_create_ddl(Some(&view.name)));
            } else {
                let (mut alter_ddl, mut comment_ddl) = self.build_rule_alter_ddl();
                ddl.append(&mut alter_ddl);
                ddl.append(&mut comment_ddl);
                let mut rules_ddl = self.build_advanced_alter_ddl();
                ddl.append(&mut rules_ddl);

                if let Some(comment) = self.build_comment_alter_ddl() {
                    ddl.push(comment);
                }
            }
        }
        ddl.join("\n")
    }
    fn build_rule_alter_ddl(&self) -> (Vec<String>, Vec<String>) {
        let mut ddl = Vec::new();
        let mut comments_ddl = Vec::new();

        let view = self.view.as_ref().unwrap();
        let rule_ids = self
            .rules
            .iter()
            .map(|r| r.id().to_owned())
            .collect::<Vec<Uuid>>();
        let old_rule_ids = view
            .rules
            .iter()
            .map(|r| r.id().to_owned())
            .collect::<Vec<Uuid>>();
        let mut drop_rule_ddl: Vec<String> = view
            .rules
            .iter()
            .filter(|r| !rule_ids.contains(r.id()))
            .map(|r| r.get_drop_ddl(view.name.as_str()))
            .collect();
        ddl.append(&mut drop_rule_ddl);
        self.rules.iter().for_each(|rule| {
            if !old_rule_ids.contains(rule.id()) {
                ddl.push(
                    rule.get_add_ddl(self.schema_name.as_deref().unwrap(), view.name.as_str()),
                );
            } else {
                let same_rule = view.rules.iter().find(|r| r.id() == r.id()).unwrap();
                let (mut alter_ddl, comment_ddl) = rule.get_alter_ddl(
                    same_rule,
                    self.schema_name.as_deref().unwrap(),
                    view.name.as_str(),
                );
                if !alter_ddl.is_empty() {
                    ddl.append(&mut alter_ddl);
                }
                if let Some(comment) = comment_ddl {
                    comments_ddl.push(comment);
                }
            }
        });
        (ddl, comments_ddl)
    }
    fn build_advanced_alter_ddl(&self) -> Vec<String> {
        let mut ddl = Vec::new();
        if self.view.as_ref().unwrap().owner != self.form.get_value("Owner") {
            ddl.push(format!(
                "ALTER VIEW \"{}\".\"{}\" OWNER TO {}",
                self.schema_name.as_deref().unwrap(),
                self.view.as_ref().unwrap().name.as_str(),
                if let Some(owner) = self.form.get_value("Owner") {
                    owner
                } else {
                    "CURRENT_ROLE".to_string()
                },
            ));
        }
        ddl
    }
    fn build_comment_alter_ddl(&self) -> Option<String> {
        let comment = self.comment.lines().join("\n");
        if self.view.as_ref().unwrap().comment != comment {
            Some(format!(
                "COMMENT ON VIEW \"{}\".\"{}\" IS '{}'",
                self.schema_name.as_deref().unwrap(),
                self.view.as_ref().unwrap().name.as_str(),
                comment,
            ))
        } else {
            None
        }
    }
    fn build_sql(&self, view_name: Option<&str>) -> String {
        if self.view.is_some() {
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
    pub fn get_main_commands(&self) -> Vec<Command> {
        let mut cmds = match self.panel {
            PanelKind::Definition => self.get_panel_definition_commands(),
            PanelKind::Rules => self.get_panel_rules_commands(),
            PanelKind::Advanced => self.get_panel_advanced_commands(),
            PanelKind::Comment => self.get_panel_comment_commands(),
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
    fn get_panel_rules_commands(&self) -> Vec<Command> {
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
                name: "New Rule",
                key: NEW_KEY,
            },
        ];
        if self.rules_state.selected().is_some() {
            cmds.push(Command {
                name: "Edit Rule",
                key: CONFIRM_KEY,
            });
            cmds.push(Command {
                name: "Delete Rule",
                key: DELETE_KEY,
            });
        }
        cmds
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
