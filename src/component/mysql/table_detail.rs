use crate::{
    app::{ComponentResult, DialogResult, MainPanel},
    component::{get_table_down_index, get_table_up_index, Command, CommandBarComponent},
    dialog::{
        confirm::{ConfirmDialog, Kind as ConfirmKind},
        mysql::{CheckDialog, FieldDialog, ForeignKeyDialog, IndexDialog, TriggerDialog},
        InputDialog,
    },
    event::{config::*, Key},
    model::mysql::{
        convert_show_column_to_mysql_fields, convert_show_fk_to_mysql_fk,
        convert_show_index_to_mysql_indexes, get_mysql_version, BinaryField, CharField, Check,
        Connections, DateField, DateTimeField, DecimalField, EnumField, Field, FieldKind,
        FloatField, ForeignKey, Index, IndexField, IndexKind, IndexMethod, IntField, OnDeleteKind,
        OnUpdateKind, SimpleField, TextField, TimeField, Trigger, TriggerAction, TriggerTime,
        Version,
    },
    pool::{execute_mysql_query_unprepared, fetch_mysql_query, fetch_one_mysql, MySQLPools},
    widget::{Form, FormItem, Select},
};
use anyhow::Result;
use regex::Regex;
use sqlx::Row;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use strum::IntoEnumIterator;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::Spans,
    widgets::{Block, BorderType, Borders, Row as RowUI, Table, TableState, Tabs},
    Frame,
};
use tui_textarea::{Input, TextArea};
use uuid::Uuid;

pub enum PanelKind {
    Fields,
    Indexes,
    ForeignKeys,
    Triggers,
    Checks,
    Options,
    Comment,
    SQLPreview,
}

pub struct TableDetailComponent<'a> {
    conn_id: Option<Uuid>,
    db_name: Option<String>,
    db_version: Version,
    table_name: Option<String>,
    panel: PanelKind,
    fields: Vec<Field>,
    old_fields: Vec<Field>,
    indexes: Vec<Index>,
    old_indexes: Vec<Index>,
    foreign_keys: Vec<ForeignKey>,
    old_foreign_keys: Vec<ForeignKey>,
    triggers: Vec<Trigger>,
    old_triggers: Vec<Trigger>,
    checks: Vec<Check>,
    old_checks: Vec<Check>,
    form: Form<'a>,
    old_form: Form<'a>,
    comment: TextArea<'a>,
    old_comment: TextArea<'a>,
    sql_preview: TextArea<'a>,
    fields_state: TableState,
    indexes_state: TableState,
    foreign_keys_state: TableState,
    triggers_state: TableState,
    checks_state: TableState,
    exit_dlg: Option<ConfirmDialog>,
    input_dlg: Option<InputDialog<'a>>,
    info_dlg: Option<ConfirmDialog>,
    delete_field_dlg: Option<ConfirmDialog>,
    delete_index_dlg: Option<ConfirmDialog>,
    delete_foreign_key_dlg: Option<ConfirmDialog>,
    delete_trigger_dlg: Option<ConfirmDialog>,
    delete_check_dlg: Option<ConfirmDialog>,
    kind_sel: Option<Select>,
    field_dlg: Option<FieldDialog<'a>>,
    index_dlg: Option<IndexDialog<'a>>,
    foreign_key_dlg: Option<ForeignKeyDialog<'a>>,
    trigger_dlg: Option<TriggerDialog<'a>>,
    check_dlg: Option<CheckDialog<'a>>,
    cmd_bar: Rc<RefCell<CommandBarComponent>>,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<MySQLPools>>,
}

impl<'a> TableDetailComponent<'a> {
    pub fn new(
        cmd_bar: Rc<RefCell<CommandBarComponent>>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<MySQLPools>>,
    ) -> Self {
        TableDetailComponent {
            table_name: None,
            panel: PanelKind::Fields,
            fields: Vec::new(),
            old_fields: Vec::new(),
            indexes: Vec::new(),
            old_indexes: Vec::new(),
            foreign_keys: Vec::new(),
            old_foreign_keys: Vec::new(),
            triggers: Vec::new(),
            old_triggers: Vec::new(),
            checks: Vec::new(),
            old_checks: Vec::new(),
            form: Form::default(),
            old_form: Form::default(),
            conn_id: None,
            db_name: None,
            db_version: Version::Eight,
            comment: TextArea::default(),
            old_comment: TextArea::default(),
            sql_preview: TextArea::default(),
            input_dlg: None,
            exit_dlg: None,
            info_dlg: None,
            delete_field_dlg: None,
            delete_index_dlg: None,
            delete_foreign_key_dlg: None,
            delete_trigger_dlg: None,
            delete_check_dlg: None,
            fields_state: TableState::default(),
            indexes_state: TableState::default(),
            foreign_keys_state: TableState::default(),
            triggers_state: TableState::default(),
            checks_state: TableState::default(),
            field_dlg: None,
            index_dlg: None,
            foreign_key_dlg: None,
            trigger_dlg: None,
            check_dlg: None,
            kind_sel: None,
            cmd_bar,
            conns,
            pools,
        }
    }
    pub async fn set_data(
        &mut self,
        conn_id: &Uuid,
        db_name: &str,
        table_name: Option<&str>,
    ) -> Result<()> {
        self.conn_id = Some(*conn_id);
        self.db_name = Some(db_name.to_string());
        self.table_name = table_name.map(|s| s.to_string());
        self.db_version =
            get_mysql_version(self.conns.clone(), self.pools.clone(), conn_id).await?;

        let engines = fetch_mysql_query(
            self.conns.clone(),
            self.pools.clone(),
            conn_id,
            None,
            "SHOW ENGINES",
        )
        .await?;
        let engines: Vec<String> = engines.iter().map(|e| e.try_get(0).unwrap()).collect();
        let charsets = fetch_mysql_query(
            self.conns.clone(),
            self.pools.clone(),
            conn_id,
            Some(db_name),
            "SHOW CHARSET",
        )
        .await?;
        let charsets: Vec<String> = charsets
            .iter()
            .map(|cs| cs.try_get("Charset").unwrap())
            .collect();

        if let Some(table_name) = self.table_name.as_ref() {
            let fields = fetch_mysql_query(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!("SHOW FULL COLUMNS FROM `{}`", table_name),
            )
            .await?;
            self.fields = convert_show_column_to_mysql_fields(fields);
            self.old_fields = self.fields.clone();

            let indexes = fetch_mysql_query(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!(
                    "SHOW INDEX FROM `{}` WHERE Key_name != 'PRIMARY'",
                    table_name
                ),
            )
            .await?;
            self.indexes = convert_show_index_to_mysql_indexes(indexes);
            self.old_indexes = self.indexes.clone();

            let foreign_keys = fetch_mysql_query(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some("information_schema"),
                format!("
                    SELECT 
                     K.CONSTRAINT_NAME,
                     K.COLUMN_NAME,
                     K.REFERENCED_TABLE_SCHEMA,
                     K.REFERENCED_TABLE_NAME,
                     K.REFERENCED_COLUMN_NAME
                    FROM
                     KEY_COLUMN_USAGE AS K
                    JOIN
                     TABLE_CONSTRAINTS AS T ON K.CONSTRAINT_NAME = T.CONSTRAINT_NAME
                    WHERE
                     K.TABLE_SCHEMA = '{}' AND K.TABLE_NAME = '{}' AND T.CONSTRAINT_TYPE = 'FOREIGN KEY'",
                    db_name, table_name
                )
                .as_str(),
            )
            .await?;
            self.foreign_keys = convert_show_fk_to_mysql_fk(foreign_keys);
            self.old_foreign_keys = self.foreign_keys.clone();

            let triggers = fetch_mysql_query(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                None,
                &format!("SHOW TRIGGERS FROM `{}` LIKE '{}' ", db_name, table_name),
            )
            .await?;
            self.triggers = triggers
                .iter()
                .map(|row| Trigger {
                    id: Uuid::new_v4(),
                    name: row.try_get("Trigger").unwrap(),
                    time: TriggerTime::try_from(
                        row.try_get::<String, _>("Timing").unwrap().as_str(),
                    )
                    .unwrap(),
                    action: TriggerAction::try_from(
                        row.try_get::<String, _>("Event").unwrap().as_str(),
                    )
                    .unwrap(),
                    statement: row.try_get("Statement").unwrap(),
                })
                .collect();
            self.old_triggers = self.triggers.clone();
            if self.db_version == Version::Eight {
                let checks = fetch_mysql_query(
                    self.conns.clone(),
                    self.pools.clone(),
                    conn_id,
                    Some("information_schema"),
                    &format!(
                        "
                    SELECT 
                    C.CONSTRAINT_NAME,
                    C.CHECK_CLAUSE,
                    T.ENFORCED
                    FROM
                    CHECK_CONSTRAINTS AS C
                    JOIN TABLE_CONSTRAINTS AS T ON C.CONSTRAINT_NAME = T.CONSTRAINT_NAME 
                    WHERE C.CONSTRAINT_SCHEMA='{}' AND T.TABLE_NAME='{}'",
                        db_name, table_name
                    ),
                )
                .await?;
                self.checks = checks
                    .iter()
                    .map(|row| Check {
                        id: Uuid::new_v4(),
                        name: row.try_get("CONSTRAINT_NAME").unwrap(),
                        expression: row
                            .try_get::<String, _>("CHECK_CLAUSE")
                            .unwrap()
                            .replace(['(', ')'], ""),
                        not_enforced: row.try_get::<String, _>("ENFORCED").unwrap() == "NO",
                    })
                    .collect();
                self.old_checks = self.checks.clone();
            }

            let create_table = fetch_one_mysql(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!("SHOW CREATE TABLE `{}`", table_name),
            )
            .await?;
            let def: String = create_table.try_get(1).unwrap();

            let reg = Regex::new(
                r"CREATE\s+TABLE\s+`\w+`\s+\(\s*(.+\s)+\)\s*(ENGINE=(?P<engine>\w+)\s*)?((DEFAULT\s+)?CHARSET=(?P<charset>\w+)\s*)?((DEFAULT\s+)?COLLATE=(?P<collation>\w+)\s*)?(MIN_ROWS=(?P<min_rows>\w+)\s*)?(MAX_ROWS=(?P<max_rows>\w+)\s*)?(AVG_ROW_LENGTH=(?P<avg_row>\w+)\s*)?(KEY_BLOCK_SIZE=(?P<kbs>\w+)\s*)?(COMMENT='(?P<comment>\w+)'\s*)?",
            )
            .unwrap();

            let caps = reg.captures(def.as_str()).unwrap();

            let collations = if let Some(charset) = caps.name("charset") {
                fetch_mysql_query(
                    self.conns.clone(),
                    self.pools.clone(),
                    self.conn_id.as_ref().unwrap(),
                    None,
                    format!("SHOW COLLATION WHERE Charset='{}'", charset.as_str()).as_str(),
                )
                .await?
                .iter()
                .map(|row| row.try_get("Collation").unwrap())
                .collect::<Vec<String>>()
            } else {
                Vec::new()
            };

            self.form.set_items(vec![
                FormItem::new_select(
                    "engine".to_string(),
                    engines,
                    caps.name("engine").map(|e| e.as_str().to_string()),
                    true,
                    false,
                ),
                FormItem::new_select(
                    "default character set".to_string(),
                    charsets,
                    caps.name("charset").map(|c| c.as_str().to_string()),
                    true,
                    false,
                ),
                FormItem::new_select(
                    "default collation".to_string(),
                    collations,
                    caps.name("collation").map(|c| c.as_str().to_string()),
                    true,
                    false,
                ),
                FormItem::new_input(
                    "avg row length".to_string(),
                    caps.name("avg_row").map(|a| a.as_str()),
                    true,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "min rows".to_string(),
                    caps.name("min_rows").map(|m| m.as_str()),
                    true,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "max rows".to_string(),
                    caps.name("max_rows").map(|m| m.as_str()),
                    true,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "key block size".to_string(),
                    caps.name("kbs").map(|k| k.as_str()),
                    true,
                    false,
                    false,
                ),
            ]);
            self.old_form = self.form.clone();
            if let Some(comment) = caps.name("comment") {
                self.comment = TextArea::from([comment.as_str()]);
                self.old_comment = self.comment.clone();
            }
        } else {
            self.form.set_items(vec![
                FormItem::new_select("engine".to_string(), engines, None, true, false),
                FormItem::new_select(
                    "default character set".to_string(),
                    charsets,
                    None,
                    true,
                    false,
                ),
                FormItem::new_select(
                    "default collation".to_string(),
                    Vec::new(),
                    None,
                    true,
                    false,
                ),
                FormItem::new_input("avg row length".to_string(), None, true, false, false),
                FormItem::new_input("min rows".to_string(), None, true, false, false),
                FormItem::new_input("max rows".to_string(), None, true, false, false),
                FormItem::new_input("key block size".to_string(), None, true, false, false),
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
                .title(if let Some(name) = &self.table_name {
                    format!("Edit `{name}`")
                } else {
                    "New Table".to_string()
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

        let select_tab = match self.db_version {
            Version::Eight => match self.panel {
                PanelKind::Fields => 0,
                PanelKind::Indexes => 1,
                PanelKind::ForeignKeys => 2,
                PanelKind::Triggers => 3,
                PanelKind::Checks => 4,
                PanelKind::Options => 5,
                PanelKind::Comment => 6,
                PanelKind::SQLPreview => 7,
            },
            Version::Five => match self.panel {
                PanelKind::Fields => 0,
                PanelKind::Indexes => 1,
                PanelKind::ForeignKeys => 2,
                PanelKind::Triggers => 3,
                PanelKind::Options => 4,
                PanelKind::Comment => 5,
                PanelKind::SQLPreview => 6,
                _ => 0,
            },
        };
        let tabs = match self.db_version {
            Version::Eight => vec![
                Spans::from("Fields"),
                Spans::from("Indexes"),
                Spans::from("Foreign Keys"),
                Spans::from("Triggers"),
                Spans::from("Checks"),
                Spans::from("Options"),
                Spans::from("Comment"),
                Spans::from("SQL Preview"),
            ],
            Version::Five => vec![
                Spans::from("Fields"),
                Spans::from("Indexes"),
                Spans::from("Foreign Keys"),
                Spans::from("Triggers"),
                Spans::from("Options"),
                Spans::from("Comment"),
                Spans::from("SQL Preview"),
            ],
        };
        f.render_widget(
            Tabs::new(tabs)
                .block(Block::default().borders(Borders::BOTTOM))
                .highlight_style(Style::default().fg(Color::Green))
                .select(select_tab),
            chunks[0],
        );
        match self.panel {
            PanelKind::Fields => self.draw_fields(f, chunks[1]),
            PanelKind::Indexes => self.draw_indexes(f, chunks[1]),
            PanelKind::ForeignKeys => self.draw_foreign_keys(f, chunks[1]),
            PanelKind::Triggers => self.draw_triggers(f, chunks[1]),
            PanelKind::Checks => self.draw_checks(f, chunks[1]),
            PanelKind::Options => self.draw_options(f, chunks[1]),
            PanelKind::Comment => self.draw_comment(f, chunks[1]),
            PanelKind::SQLPreview => self.draw_sql_preview(f, chunks[1]),
        }

        if is_focus {
            self.update_commands();
        }
        Ok(())
    }
    pub fn draw_dialog<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        if let Some(sel) = self.kind_sel.as_mut() {
            sel.draw(f);
        }
        if let Some(dlg) = self.input_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.exit_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.info_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.field_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.index_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.foreign_key_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.trigger_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.check_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.delete_field_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.delete_index_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.delete_foreign_key_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.delete_trigger_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.delete_check_dlg.as_mut() {
            dlg.draw(f);
        }
    }
    pub async fn handle_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if self.input_dlg.is_some() {
            self.handle_input_dlg_event(key).await
        } else if self.exit_dlg.is_some() {
            self.handle_exit_dlg_event(key)
        } else if self.info_dlg.is_some() {
            self.handle_info_dlg_event(key)
        } else if self.kind_sel.is_some() {
            self.handle_kind_select_event(key).await
        } else if self.delete_field_dlg.is_some() {
            self.handle_delete_field_event(key)
        } else if self.delete_index_dlg.is_some() {
            self.handle_delete_index_event(key)
        } else if self.delete_check_dlg.is_some() {
            self.handle_delete_check_event(key)
        } else if self.delete_trigger_dlg.is_some() {
            self.handle_delete_trigger_event(key)
        } else if self.delete_foreign_key_dlg.is_some() {
            self.handle_delete_foreign_key_event(key)
        } else if self.field_dlg.is_some() {
            self.handle_field_dlg_event(key).await
        } else if self.index_dlg.is_some() {
            self.handle_index_dlg_event(key)
        } else if self.foreign_key_dlg.is_some() {
            self.handle_foreign_key_dlg_event(key).await
        } else if self.trigger_dlg.is_some() {
            self.handle_trigger_dlg_event(key)
        } else if self.check_dlg.is_some() {
            self.handle_check_dlg_event(key)
        } else {
            self.handle_main_event(key).await
        }
    }

    fn draw_fields<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(
            self.fields
                .iter()
                .map(|f| {
                    RowUI::new(vec![
                        f.name(),
                        f.kind_str(),
                        if f.not_null() { "\u{2705}" } else { "" },
                        if f.key() { "\u{2705}" } else { "" },
                        f.default_value().unwrap_or_default(),
                        f.extra().unwrap_or_default(),
                        f.comment().unwrap_or_default(),
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec![
            "Name",
            "Type",
            "Not Null",
            "Key",
            "Default Value",
            "Extra",
            "Comment",
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
        f.render_stateful_widget(table, r, &mut self.fields_state);
    }

    fn build_sql(&self, table_name: Option<&str>) -> String {
        if self.table_name.is_some() {
            self.build_alter_ddl()
        } else {
            self.build_create_ddl(table_name)
        }
    }
    fn build_create_ddl(&self, table_name: Option<&str>) -> String {
        let table_name = if let Some(name) = table_name {
            name
        } else {
            "new table"
        };

        let mut ddl_sql = vec![];
        let mut field_sqls = self
            .fields
            .iter()
            .map(|f| f.get_create_str())
            .collect::<Vec<String>>();
        ddl_sql.append(&mut field_sqls);

        let key_fields: Vec<&Field> = self.fields.iter().filter(|f| f.key()).collect();
        if !key_fields.is_empty() {
            let names: Vec<String> = key_fields
                .iter()
                .map(|f| format!("`{}`", f.name()))
                .collect();

            ddl_sql.push(format!("\nPRIMARY KEY ({})", names.join(",")));
        }
        let mut index_sqls: Vec<String> = self
            .indexes
            .iter()
            .map(|index| index.get_create_ddl())
            .collect();
        if !index_sqls.is_empty() {
            ddl_sql.append(&mut index_sqls);
        }

        let mut fk_sqls: Vec<String> = self
            .foreign_keys
            .iter()
            .map(|fk| fk.get_create_ddl())
            .collect();
        if !fk_sqls.is_empty() {
            ddl_sql.append(&mut fk_sqls);
        }
        let mut check_sqls: Vec<String> = self
            .checks
            .iter()
            .map(|check| check.get_create_ddl())
            .collect();
        if !check_sqls.is_empty() {
            ddl_sql.append(&mut check_sqls);
        }

        let trigger_sqls = if !self.triggers.is_empty() {
            self.triggers
                .iter()
                .map(|t| t.get_create_ddl(table_name))
                .collect()
        } else {
            vec![]
        };

        format!(
            "CREATE TABLE `{}` (\n{}\n){};\n{}",
            table_name,
            ddl_sql.join(",\n"),
            self.build_options_sql(),
            trigger_sqls.join("\n")
        )
    }
    fn build_alter_ddl(&self) -> String {
        let mut ddl: Vec<String> = Vec::new();
        let mut alter_ddl = Vec::new();
        alter_ddl.extend(self.build_field_alter_ddl());
        alter_ddl.extend(self.build_index_alter_ddl());
        alter_ddl.extend(self.build_foreign_key_alter_ddl());
        alter_ddl.extend(self.build_check_alter_ddl());
        let alter_option_ddl = self.build_options_alter_ddl();
        if !alter_option_ddl.is_empty() {
            alter_ddl.push(alter_option_ddl);
        }

        if !alter_ddl.is_empty() {
            ddl.push(format!(
                "ALTER TABLE `{}`\n{};",
                self.table_name.as_ref().unwrap(),
                alter_ddl.join(",\n")
            ));
        }

        ddl.extend(self.build_trigger_alter_ddl());
        ddl.join("\n")
    }
    fn build_field_alter_ddl(&self) -> Vec<String> {
        let mut ddl = Vec::new();
        let field_ids = self
            .fields
            .iter()
            .map(|f| f.id().to_owned())
            .collect::<Vec<Uuid>>();
        let old_field_ids = self
            .old_fields
            .iter()
            .map(|f| f.id().to_owned())
            .collect::<Vec<Uuid>>();
        let mut drop_fields_str = self
            .old_fields
            .iter()
            .filter(|field| !field_ids.contains(field.id()))
            .map(|field| field.get_drop_str())
            .collect();
        self.fields.iter().for_each(|field| {
            if !old_field_ids.contains(field.id()) {
                ddl.push(field.get_add_str());
            } else {
                let same_field = self
                    .old_fields
                    .iter()
                    .find(|f| f.id() == field.id())
                    .unwrap();
                if let Some(str) = field.get_change_str(same_field) {
                    ddl.push(str);
                }
            }
        });
        ddl.append(&mut drop_fields_str);
        let old_key_fields = self
            .old_fields
            .iter()
            .filter(|f| f.key())
            .map(|f| f.name().to_string())
            .collect::<Vec<String>>();
        let key_fields = self
            .fields
            .iter()
            .filter(|f| f.key())
            .map(|f| f.name().to_string())
            .collect::<Vec<String>>();
        if old_key_fields != key_fields {
            if !old_key_fields.is_empty() {
                ddl.push(format!("DROP PRIMARY KEY"));
            }
            ddl.push(format!(
                "ADD PRIMARY KEY ({})",
                key_fields
                    .iter()
                    .map(|f| format!("`{}`", f))
                    .collect::<Vec<String>>()
                    .join(",")
            ));
        }
        ddl
    }
    fn build_index_alter_ddl(&self) -> Vec<String> {
        let mut ddl = Vec::new();
        let ids = self
            .indexes
            .iter()
            .map(|i| i.id().to_owned())
            .collect::<Vec<Uuid>>();
        let old_ids = self
            .old_indexes
            .iter()
            .map(|i| i.id().to_owned())
            .collect::<Vec<Uuid>>();
        let mut drop_indexes_str = self
            .old_indexes
            .iter()
            .filter(|index| !ids.contains(index.id()))
            .map(|index| index.get_drop_ddl())
            .collect();
        self.indexes.iter().for_each(|index| {
            if !old_ids.contains(index.id()) {
                ddl.push(index.get_add_ddl());
            } else {
                let same_index = self
                    .old_indexes
                    .iter()
                    .find(|i| i.id() == index.id())
                    .unwrap();
                ddl.extend(index.get_alter_ddl(same_index));
            }
        });
        ddl.append(&mut drop_indexes_str);
        ddl
    }
    fn build_foreign_key_alter_ddl(&self) -> Vec<String> {
        let mut ddl = Vec::new();
        let ids = self
            .foreign_keys
            .iter()
            .map(|f| f.id().to_owned())
            .collect::<Vec<Uuid>>();
        let old_ids = self
            .old_foreign_keys
            .iter()
            .map(|f| f.id().to_owned())
            .collect::<Vec<Uuid>>();
        let mut drop_foreign_key_str = self
            .old_foreign_keys
            .iter()
            .filter(|k| !ids.contains(k.id()))
            .map(|k| k.get_drop_ddl())
            .collect();
        self.foreign_keys.iter().for_each(|key| {
            if !old_ids.contains(key.id()) {
                ddl.push(key.get_add_ddl());
            } else {
                let same_fk = self
                    .old_foreign_keys
                    .iter()
                    .find(|fk| fk.id() == key.id())
                    .unwrap();
                ddl.extend(key.get_alter_ddl(same_fk));
            }
        });
        ddl.append(&mut drop_foreign_key_str);
        ddl
    }
    fn build_trigger_alter_ddl(&self) -> Vec<String> {
        let mut ddl = Vec::new();
        let ids = self
            .triggers
            .iter()
            .map(|t| t.id().to_owned())
            .collect::<Vec<Uuid>>();
        let old_ids = self
            .old_triggers
            .iter()
            .map(|t| t.id().to_owned())
            .collect::<Vec<Uuid>>();
        let mut drop_trigger_str = self
            .old_triggers
            .iter()
            .filter(|t| !ids.contains(t.id()))
            .map(|t| t.get_drop_ddl())
            .collect();
        self.triggers.iter().for_each(|trigger| {
            if !old_ids.contains(trigger.id()) {
                ddl.push(trigger.get_create_ddl(self.table_name.as_deref().unwrap()));
            } else {
                let same_trigger = self
                    .old_triggers
                    .iter()
                    .find(|t| t.id() == trigger.id())
                    .unwrap();
                ddl.extend(
                    trigger.get_alter_ddl(same_trigger, self.table_name.as_deref().unwrap()),
                );
            }
        });
        ddl.append(&mut drop_trigger_str);
        ddl
    }
    fn build_check_alter_ddl(&self) -> Vec<String> {
        let mut ddl = Vec::new();
        let ids = self
            .checks
            .iter()
            .map(|c| c.id().to_owned())
            .collect::<Vec<Uuid>>();
        let old_ids = self
            .old_checks
            .iter()
            .map(|c| c.id().to_owned())
            .collect::<Vec<Uuid>>();
        let mut drop_check_str = self
            .old_checks
            .iter()
            .filter(|c| !ids.contains(c.id()))
            .map(|c| c.get_drop_ddl())
            .collect();
        self.checks.iter().for_each(|check| {
            if !old_ids.contains(check.id()) {
                ddl.push(check.get_add_ddl());
            } else {
                let same_check = self
                    .old_checks
                    .iter()
                    .find(|old_check| old_check.id() == check.id())
                    .unwrap();
                if let Some(str) = check.get_change_ddl(same_check) {
                    ddl.push(str);
                }
            }
        });
        ddl.append(&mut drop_check_str);
        ddl
    }

    fn build_options_alter_ddl(&self) -> String {
        let mut str = String::new();

        let engine = self.form.get_value("engine");
        if self.old_form.get_value("engine") != engine {
            if let Some(e) = engine {
                str.push_str(&format!(" ENGINE = {}", e));
            }
        }

        let charset = self.form.get_value("default character set");
        if self.old_form.get_value("default character set") != charset {
            if let Some(d) = charset {
                str.push_str(&format!(" DEFAULT CHARACTER SET = {}", d));
            }
        }

        let collate = self.form.get_value("default collation");
        if self.old_form.get_value("default collation") != collate {
            if let Some(d) = collate {
                str.push_str(&format!(" DEFAULT COLLATE = {}", d));
            }
        }

        let avg_row_length = self.form.get_value("avg row length");
        if self.old_form.get_value("avg row length") != avg_row_length {
            str.push_str(&format!(
                " AVG_ROW_LENGTH = {}",
                avg_row_length.unwrap_or(String::from("0"))
            ));
        }

        let min_rows = self.form.get_value("min rows");
        if self.old_form.get_value("min rows") != min_rows {
            str.push_str(&format!(
                " MIN_ROWS = {}",
                min_rows.unwrap_or(String::from("0"))
            ));
        }
        let max_rows = self.form.get_value("max rows");
        if self.old_form.get_value("max rows") != max_rows {
            str.push_str(&format!(
                " MAX_ROWS = {}",
                max_rows.unwrap_or(String::from("0"))
            ));
        }

        let key_block_size = self.form.get_value("key block size");
        if self.old_form.get_value("key block size") != key_block_size {
            str.push_str(&format!(
                " KEY_BLOCK_SIZE = {}",
                key_block_size.unwrap_or(String::from("0"))
            ));
        }

        let comment = self.comment.lines().join("\n");
        let old_comment = self.old_comment.lines().join("\n");
        if comment != old_comment {
            str.push_str(&format!(" COMMENT = '{}'", comment));
        }
        str
    }
    fn build_options_sql(&self) -> String {
        let map = self.form.get_data();
        let mut sql = "".to_string();
        if let Some(engine) = map.get("engine").unwrap() {
            if !engine.is_empty() {
                sql = format!("{}ENGINE {} ", sql, engine);
            }
        }
        if let Some(avl) = map.get("avg row length").unwrap() {
            if !avl.is_empty() {
                sql = format!("{}AVG_ROW_LENGTH = {} ", sql, avl);
            }
        }
        if let Some(dcs) = map.get("default character set").unwrap() {
            if !dcs.is_empty() {
                sql = format!("{}CHARACTER SET = {} ", sql, dcs);
            }
        }
        if let Some(dc) = map.get("default collation").unwrap() {
            if !dc.is_empty() {
                sql = format!("{}COLLATE {} ", sql, dc);
            }
        }
        if let Some(kbs) = map.get("key block size").unwrap() {
            if !kbs.is_empty() {
                sql = format!("{}KEY_BLOCK_SIZE = {} ", sql, kbs);
            }
        }
        if let Some(mr) = map.get("max rows").unwrap() {
            if !mr.is_empty() {
                sql = format!("{}MAX_ROWS = {}", sql, mr);
            }
        }
        if let Some(mr) = map.get("min rows").unwrap() {
            if !mr.is_empty() {
                sql = format!("{}MIN_ROWS = {}", sql, mr);
            }
        }
        if !self.comment.is_empty() {
            sql = format!("{}COMMENT = '{}'", sql, self.comment.lines().join("\n"));
        }
        sql
    }

    fn draw_indexes<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(
            self.indexes
                .iter()
                .map(|i| {
                    RowUI::new(vec![
                        i.name.clone(),
                        i.fields
                            .iter()
                            .map(|f| f.to_string())
                            .collect::<Vec<String>>()
                            .join(","),
                        i.kind.to_string(),
                        i.method.clone().map(|s| s.to_string()).unwrap_or_default(),
                        i.comment.clone().unwrap_or_default(),
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec![
            "Name",
            "Fields",
            "Index Type",
            "Method",
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
        f.render_stateful_widget(table, r, &mut self.indexes_state);
    }
    fn draw_foreign_keys<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(
            self.foreign_keys
                .iter()
                .map(|f| {
                    RowUI::new(vec![
                        f.name(),
                        f.field(),
                        f.ref_table(),
                        f.ref_field(),
                        f.on_delete().unwrap_or(""),
                        f.on_update().unwrap_or(""),
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec![
            "Name",
            "Fields",
            "Ref Table",
            "Ref Fields",
            "On Delete",
            "On Update",
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
        f.render_stateful_widget(table, r, &mut self.foreign_keys_state);
    }
    fn draw_triggers<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(
            self.triggers
                .iter()
                .map(|t| RowUI::new(vec![t.name(), t.time(), t.action(), t.statement()]))
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec!["Name", "Time", "Action", "Statement"]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(table, r, &mut self.triggers_state);
    }
    fn draw_checks<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(
            self.checks
                .iter()
                .map(|c| {
                    RowUI::new(vec![
                        c.name(),
                        c.expression(),
                        if c.not_enforced() { "\u{2705}" } else { "" },
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec!["Name", "Expression", "Not Enforced"]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(table, r, &mut self.checks_state);
    }
    fn draw_options<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
                Constraint::Ratio(1, 3),
            ])
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
    pub fn clear(&mut self) {
        self.conn_id = None;
        self.db_name = None;
        self.db_version = Version::Eight;
        self.table_name = None;
        self.panel = PanelKind::Fields;
        self.fields = Vec::new();
        self.old_fields = Vec::new();
        self.indexes = Vec::new();
        self.old_indexes = Vec::new();
        self.foreign_keys = Vec::new();
        self.old_foreign_keys = Vec::new();
        self.triggers = Vec::new();
        self.old_triggers = Vec::new();
        self.checks = Vec::new();
        self.old_checks = Vec::new();
        self.form = Form::default();
        self.old_form = Form::default();
        self.comment = TextArea::default();
        self.old_comment = TextArea::default();
        self.fields_state = TableState::default();
        self.indexes_state = TableState::default();
        self.foreign_keys_state = TableState::default();
        self.triggers_state = TableState::default();
        self.checks_state = TableState::default();
        self.exit_dlg = None;
        self.input_dlg = None;
        self.delete_field_dlg = None;
        self.delete_index_dlg = None;
        self.delete_foreign_key_dlg = None;
        self.delete_trigger_dlg = None;
        self.delete_check_dlg = None;
        self.kind_sel = None;
        self.field_dlg = None;
        self.index_dlg = None;
        self.foreign_key_dlg = None;
        self.trigger_dlg = None;
        self.check_dlg = None;
    }
    async fn handle_kind_select_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(select) = self.kind_sel.as_mut() {
            match select.handle_event(key) {
                DialogResult::Cancel => {
                    self.kind_sel = None;
                }
                DialogResult::Confirm(kind) => {
                    let kind = FieldKind::try_from(kind).unwrap();
                    self.field_dlg = Some(
                        FieldDialog::new(
                            kind,
                            self.conns.clone(),
                            self.pools.clone(),
                            self.conn_id.as_ref().unwrap(),
                        )
                        .await?,
                    );
                    self.kind_sel = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_info_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.info_dlg.as_mut() {
            dlg.handle_event(key);
            self.old_fields = self.fields.clone();
            self.old_indexes = self.indexes.clone();
            self.old_foreign_keys = self.foreign_keys.clone();
            self.old_triggers = self.triggers.clone();
            self.old_checks = self.checks.clone();
            self.old_form = self.form.clone();
            self.old_comment = self.comment.clone();
            self.info_dlg = None;
        }
        Ok(ComponentResult::Done)
    }
    fn handle_exit_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.exit_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => self.exit_dlg = None,
                DialogResult::Confirm(_) => {
                    self.clear();
                    return Ok(ComponentResult::Back(MainPanel::TableListMySQL));
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_field_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.field_dlg.as_mut() {
            match dlg.handle_event(key).await? {
                DialogResult::Cancel => {
                    self.field_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    let field = Self::map_to_mysql_field(dlg.get_kind(), &map);
                    match dlg.get_id() {
                        None => self.fields.push(field),
                        Some(_) => {
                            if let Some(index) = self.fields_state.selected() {
                                self.fields.splice(index..index + 1, [field]);
                            }
                        }
                    }
                    self.field_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }

    fn handle_index_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.index_dlg.as_mut() {
            match dlg.handle_event(key)? {
                DialogResult::Cancel => {
                    self.index_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    let index = Self::map_to_index(&map);
                    match dlg.get_id() {
                        None => self.indexes.push(index),
                        Some(_) => {
                            if let Some(idx) = self.indexes_state.selected() {
                                self.indexes.splice(idx..idx + 1, [index]);
                            }
                        }
                    }
                    self.index_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }

    async fn handle_foreign_key_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.foreign_key_dlg.as_mut() {
            match dlg.handle_event(key).await? {
                DialogResult::Cancel => {
                    self.foreign_key_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    let foreign_key = Self::map_to_foreign_key(&map);
                    match dlg.get_id() {
                        None => self.foreign_keys.push(foreign_key),
                        Some(_) => {
                            if let Some(index) = self.foreign_keys_state.selected() {
                                self.foreign_keys.splice(index..index + 1, [foreign_key]);
                            }
                        }
                    }
                    self.foreign_key_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_trigger_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.trigger_dlg.as_mut() {
            match dlg.handle_event(key)? {
                DialogResult::Cancel => {
                    self.trigger_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    let trigger = Self::map_to_trigger(&map);
                    match dlg.get_id() {
                        None => self.triggers.push(trigger),
                        Some(_) => {
                            if let Some(index) = self.triggers_state.selected() {
                                self.triggers.splice(index..index + 1, [trigger]);
                            }
                        }
                    }
                    self.trigger_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_check_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.check_dlg.as_mut() {
            match dlg.handle_event(key)? {
                DialogResult::Cancel => {
                    self.check_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    let check = Self::map_to_check(&map);
                    match dlg.get_id() {
                        None => self.checks.push(check),
                        Some(_) => {
                            if let Some(index) = self.checks_state.selected() {
                                self.checks.splice(index..index + 1, [check]);
                            }
                        }
                    }
                    self.check_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn map_to_mysql_field(kind: &FieldKind, map: &HashMap<String, Option<String>>) -> Field {
        match kind {
            FieldKind::BigInt => Field::BigInt(Self::map_to_int_field(map)),
            FieldKind::Binary => Field::Binary(Self::map_to_binary_field(map)),
            FieldKind::Bit => Field::Bit(Self::map_to_binary_field(map)),
            FieldKind::Blob => Field::Blob(Self::map_to_simple_field(map)),
            FieldKind::Char => Field::Char(Self::map_to_char_field(map)),
            FieldKind::Date => Field::Date(Self::map_to_date_field(map)),
            FieldKind::DateTime => Field::DateTime(Self::map_to_datetime_field(map)),
            FieldKind::Decimal => Field::Decimal(Self::map_to_decimal_field(map)),
            FieldKind::Double => Field::Double(Self::map_to_float_field(map)),
            FieldKind::Enum => Field::Enum(Self::map_to_enum_field(map)),
            FieldKind::Float => Field::Float(Self::map_to_float_field(map)),
            FieldKind::Geometry => Field::Geometry(Self::map_to_simple_field(map)),
            FieldKind::GeometryCollection => {
                Field::GeometryCollection(Self::map_to_simple_field(map))
            }
            FieldKind::Int => Field::Int(Self::map_to_int_field(map)),
            FieldKind::Integer => Field::Integer(Self::map_to_int_field(map)),
            FieldKind::Json => Field::Json(Self::map_to_simple_field(map)),
            FieldKind::LineString => Field::LineString(Self::map_to_simple_field(map)),
            FieldKind::LongBlob => Field::LongBlob(Self::map_to_simple_field(map)),
            FieldKind::LongText => Field::LongText(Self::map_to_text_field(map)),
            FieldKind::MediumBlob => Field::MediumBlob(Self::map_to_simple_field(map)),
            FieldKind::MediumInt => Field::MediumInt(Self::map_to_int_field(map)),
            FieldKind::MediumText => Field::MediumText(Self::map_to_text_field(map)),
            FieldKind::MultiLineString => Field::MultiLineString(Self::map_to_simple_field(map)),
            FieldKind::MultiPoint => Field::MultiPoint(Self::map_to_simple_field(map)),
            FieldKind::MultiPolygon => Field::MultiPolygon(Self::map_to_simple_field(map)),
            FieldKind::Numeric => Field::Numeric(Self::map_to_decimal_field(map)),
            FieldKind::Point => Field::Point(Self::map_to_simple_field(map)),
            FieldKind::Polygon => Field::Polygon(Self::map_to_simple_field(map)),
            FieldKind::Real => Field::Real(Self::map_to_float_field(map)),
            FieldKind::Set => Field::Set(Self::map_to_enum_field(map)),
            FieldKind::SmallInt => Field::SmallInt(Self::map_to_int_field(map)),
            FieldKind::Text => Field::Text(Self::map_to_text_field(map)),
            FieldKind::Time => Field::Time(Self::map_to_time_field(map)),
            FieldKind::Timestamp => Field::Timestamp(Self::map_to_datetime_field(map)),
            FieldKind::TinyBlob => Field::TinyBlob(Self::map_to_simple_field(map)),
            FieldKind::TinyInt => Field::TinyInt(Self::map_to_int_field(map)),
            FieldKind::TinyText => Field::TinyText(Self::map_to_text_field(map)),
            FieldKind::VarBinary => Field::VarBinary(Self::map_to_binary_field(map)),
            FieldKind::VarChar => Field::VarChar(Self::map_to_char_field(map)),
            FieldKind::Year => Field::Year(Self::map_to_date_field(map)),
        }
    }
    fn map_to_int_field(map: &HashMap<String, Option<String>>) -> IntField {
        IntField {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            not_null: map.get("not null").unwrap().as_ref().unwrap() == "true",
            key: map.get("key").unwrap().as_ref().unwrap() == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
            length: map.get("length").unwrap().as_ref().map(|s| s.to_string()),
            default_value: map
                .get("default value")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            auto_increment: map.get("auto increment").unwrap().as_ref().unwrap() == "true",
            unsigned: map.get("unsigned").unwrap().as_ref().unwrap() == "true",
            zerofill: map.get("zerofill").unwrap().as_ref().unwrap() == "true",
        }
    }
    fn map_to_char_field(map: &HashMap<String, Option<String>>) -> CharField {
        CharField {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            not_null: map.get("not null").unwrap().as_ref().unwrap() == "true",
            key: map.get("key").unwrap().as_ref().unwrap() == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
            length: map.get("length").unwrap().as_ref().map(|s| s.to_string()),
            default_value: map
                .get("default value")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            character_set: map
                .get("character set")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            collation: map
                .get("collation")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
        }
    }
    fn map_to_binary_field(map: &HashMap<String, Option<String>>) -> BinaryField {
        BinaryField {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            not_null: map.get("not null").unwrap().as_ref().unwrap() == "true",
            key: map.get("key").unwrap().as_ref().unwrap() == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
            length: map.get("length").unwrap().as_ref().unwrap().to_string(),
            default_value: map
                .get("default value")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
        }
    }
    fn map_to_date_field(map: &HashMap<String, Option<String>>) -> DateField {
        DateField {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            not_null: map.get("not null").unwrap().as_ref().unwrap() == "true",
            key: map.get("key").unwrap().as_ref().unwrap() == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
            default_value: map
                .get("default value")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
        }
    }
    fn map_to_simple_field(map: &HashMap<String, Option<String>>) -> SimpleField {
        SimpleField {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            not_null: map.get("not null").unwrap().as_ref().unwrap() == "true",
            key: map.get("key").unwrap().as_ref().unwrap() == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
        }
    }
    fn map_to_decimal_field(map: &HashMap<String, Option<String>>) -> DecimalField {
        DecimalField {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            not_null: map.get("not null").unwrap().as_ref().unwrap() == "true",
            key: map.get("key").unwrap().as_ref().unwrap() == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
            length: map.get("length").unwrap().as_ref().map(|s| s.to_string()),
            decimal: map.get("decimal").unwrap().as_ref().map(|s| s.to_string()),
            default_value: map
                .get("default value")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            unsigned: map.get("unsigned").unwrap().as_ref().unwrap() == "true",
            zerofill: map.get("zerofill").unwrap().as_ref().unwrap() == "true",
        }
    }
    fn map_to_enum_field(map: &HashMap<String, Option<String>>) -> EnumField {
        EnumField {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            not_null: map.get("not null").unwrap().as_ref().unwrap() == "true",
            key: map.get("key").unwrap().as_ref().unwrap() == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
            options: map
                .get("options")
                .unwrap()
                .as_ref()
                .unwrap()
                .split(',')
                .map(|s| s.to_string())
                .collect(),
            default_value: map
                .get("default value")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            character_set: map
                .get("character set")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            collation: map
                .get("collation")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
        }
    }

    fn map_to_float_field(map: &HashMap<String, Option<String>>) -> FloatField {
        FloatField {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            not_null: map.get("not null").unwrap().as_ref().unwrap() == "true",
            key: map.get("key").unwrap().as_ref().unwrap() == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
            length: map.get("length").unwrap().as_ref().map(|s| s.to_string()),
            decimal: map.get("decimal").unwrap().as_ref().map(|s| s.to_string()),
            default_value: map
                .get("default value")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            auto_increment: map.get("auto increment").unwrap().as_ref().unwrap() == "true",
            unsigned: map.get("unsigned").unwrap().as_ref().unwrap() == "true",
            zerofill: map.get("zerofill").unwrap().as_ref().unwrap() == "true",
        }
    }
    fn map_to_datetime_field(map: &HashMap<String, Option<String>>) -> DateTimeField {
        DateTimeField {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            not_null: map.get("not null").unwrap().as_ref().unwrap() == "true",
            key: map.get("key").unwrap().as_ref().unwrap() == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
            length: map.get("length").unwrap().as_ref().map(|s| s.to_string()),
            default_value: map
                .get("default value")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            on_update: map.get("on update").unwrap().as_ref().unwrap() == "true",
        }
    }
    fn map_to_time_field(map: &HashMap<String, Option<String>>) -> TimeField {
        TimeField {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            not_null: map.get("not null").unwrap().as_ref().unwrap() == "true",
            key: map.get("key").unwrap().as_ref().unwrap() == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
            length: map.get("length").unwrap().as_ref().map(|s| s.to_string()),
            default_value: map
                .get("default value")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
        }
    }
    fn map_to_text_field(map: &HashMap<String, Option<String>>) -> TextField {
        TextField {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            not_null: map.get("not null").unwrap().as_ref().unwrap() == "true",
            key: map.get("key").unwrap().as_ref().unwrap() == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
            character_set: map
                .get("character set")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            collation: map
                .get("collation")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
        }
    }
    fn map_to_index(map: &HashMap<String, Option<String>>) -> Index {
        Index {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            fields: map
                .get("fields")
                .unwrap()
                .as_ref()
                .unwrap()
                .split(',')
                .map(|f| IndexField::try_from(f).unwrap())
                .collect(),
            kind: map
                .get("kind")
                .unwrap()
                .as_ref()
                .map(|k| IndexKind::try_from(k.as_str()).unwrap())
                .unwrap_or(IndexKind::Normal),
            method: map
                .get("method")
                .unwrap()
                .as_ref()
                .map(|method| IndexMethod::try_from(method.as_str()).unwrap()),
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
        }
    }
    fn map_to_foreign_key(map: &HashMap<String, Option<String>>) -> ForeignKey {
        ForeignKey {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            field: map.get("field").unwrap().as_ref().unwrap().to_string(),
            ref_db: map
                .get("reference db")
                .unwrap()
                .as_ref()
                .unwrap()
                .to_string(),
            ref_table: map
                .get("reference table")
                .unwrap()
                .as_ref()
                .unwrap()
                .to_string(),
            ref_field: map
                .get("reference field")
                .unwrap()
                .as_ref()
                .unwrap()
                .to_string(),
            on_delete: map
                .get("on delete")
                .unwrap()
                .as_ref()
                .map(|od| OnDeleteKind::try_from(od.as_str()).unwrap()),
            on_update: map
                .get("on update")
                .unwrap()
                .as_ref()
                .map(|ou| OnUpdateKind::try_from(ou.as_str()).unwrap()),
        }
    }
    fn map_to_trigger(map: &HashMap<String, Option<String>>) -> Trigger {
        Trigger {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            time: TriggerTime::try_from(map.get("time").unwrap().as_ref().unwrap().as_str())
                .unwrap(),
            action: TriggerAction::try_from(map.get("action").unwrap().as_ref().unwrap().as_str())
                .unwrap(),
            statement: map.get("statement").unwrap().as_ref().unwrap().to_string(),
        }
    }

    fn map_to_check(map: &HashMap<String, Option<String>>) -> Check {
        Check {
            id: if let Some(id) = map.get("id") {
                if let Some(id) = id {
                    Uuid::parse_str(id).unwrap()
                } else {
                    Uuid::new_v4()
                }
            } else {
                Uuid::new_v4()
            },
            name: map.get("name").unwrap().as_ref().unwrap().to_string(),
            expression: map.get("expression").unwrap().as_ref().unwrap().to_string(),
            not_enforced: map.get("not enforced").unwrap().as_ref().unwrap() == "true",
        }
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
        if self.table_name.is_some() {
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
            self.input_dlg = Some(InputDialog::new("Table Name", None));
        }

        Ok(ComponentResult::Done)
    }
    async fn handle_main_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match self.panel {
            PanelKind::Fields => self.handle_panel_fields_event(key).await,
            PanelKind::Indexes => self.handle_panel_indexes_event(key).await,
            PanelKind::ForeignKeys => self.handle_panel_foreign_keys_event(key).await,
            PanelKind::Triggers => self.handle_panel_triggers_event(key).await,
            PanelKind::Checks => self.handle_panel_checks_event(key).await,
            PanelKind::Options => self.handle_panel_options_event(key).await,
            PanelKind::Comment => self.handle_panel_comment_event(key).await,
            PanelKind::SQLPreview => self.handle_panel_sql_preview_event(key).await,
        }
    }
    async fn handle_panel_fields_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_RIGHT_KEY => {
                self.panel = PanelKind::Indexes;
            }
            TAB_LEFT_KEY => {
                self.panel = PanelKind::SQLPreview;
            }
            BACK_KEY => {
                self.handle_back_event()?;
            }
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            UP_KEY => {
                if !self.fields.is_empty() {
                    let index = get_table_up_index(self.fields_state.selected());
                    self.fields_state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.fields.is_empty() {
                    let index =
                        get_table_down_index(self.fields_state.selected(), self.fields.len());
                    self.fields_state.select(Some(index));
                }
            }
            NEW_KEY => {
                self.kind_sel = Some(Select::new(
                    "Field Type".to_string(),
                    FieldKind::iter().map(|s| s.to_string()).collect(),
                    None,
                ));
            }
            CONFIRM_KEY => {
                if let Some(index) = self.fields_state.selected() {
                    self.field_dlg = Some(
                        FieldDialog::from_field(
                            &self.fields[index],
                            self.conns.clone(),
                            self.pools.clone(),
                            self.conn_id.as_ref().unwrap(),
                        )
                        .await?,
                    );
                }
            }
            DELETE_KEY => {
                if let Some(index) = self.fields_state.selected() {
                    self.delete_field_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Warning,
                        "Delete Field",
                        &format!("Are you sure to delete {}?", self.fields[index].name()),
                    ));
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_indexes_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_RIGHT_KEY => {
                self.panel = PanelKind::ForeignKeys;
            }
            TAB_LEFT_KEY => {
                self.panel = PanelKind::Fields;
            }
            BACK_KEY => {
                self.handle_back_event()?;
            }
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            UP_KEY => {
                if !self.indexes.is_empty() {
                    let index = get_table_up_index(self.indexes_state.selected());
                    self.indexes_state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.indexes.is_empty() {
                    let index =
                        get_table_down_index(self.indexes_state.selected(), self.indexes.len());
                    self.indexes_state.select(Some(index));
                }
            }
            NEW_KEY => {
                self.index_dlg = Some(IndexDialog::new(&self.fields, &self.db_version, None));
            }
            CONFIRM_KEY => {
                if let Some(index) = self.indexes_state.selected() {
                    self.index_dlg = Some(IndexDialog::new(
                        &self.fields,
                        &self.db_version,
                        Some(&self.indexes[index]),
                    ));
                }
            }
            DELETE_KEY => {
                if self.indexes_state.selected().is_some() {
                    self.delete_index_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Warning,
                        "Delete Index",
                        "Are you sure to delete index?",
                    ));
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_foreign_keys_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_RIGHT_KEY => {
                self.panel = PanelKind::Triggers;
            }
            TAB_LEFT_KEY => {
                self.panel = PanelKind::Indexes;
            }
            BACK_KEY => {
                self.handle_back_event()?;
            }
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            UP_KEY => {
                if !self.foreign_keys.is_empty() {
                    let index = get_table_up_index(self.foreign_keys_state.selected());
                    self.foreign_keys_state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.foreign_keys.is_empty() {
                    let index = get_table_down_index(
                        self.foreign_keys_state.selected(),
                        self.foreign_keys.len(),
                    );
                    self.foreign_keys_state.select(Some(index));
                }
            }
            NEW_KEY => {
                self.foreign_key_dlg = Some(
                    ForeignKeyDialog::new(
                        &self.fields,
                        None,
                        self.conn_id.as_ref().unwrap(),
                        self.conns.clone(),
                        self.pools.clone(),
                    )
                    .await?,
                );
            }
            CONFIRM_KEY => {
                if let Some(index) = self.foreign_keys_state.selected() {
                    self.foreign_key_dlg = Some(
                        ForeignKeyDialog::new(
                            &self.fields,
                            Some(&self.foreign_keys[index]),
                            self.conn_id.as_ref().unwrap(),
                            self.conns.clone(),
                            self.pools.clone(),
                        )
                        .await?,
                    );
                }
            }
            DELETE_KEY => {
                if self.foreign_keys_state.selected().is_some() {
                    self.delete_foreign_key_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Warning,
                        "Delete Foreign Key",
                        "Are you sure to delete foreign key?",
                    ));
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_triggers_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_RIGHT_KEY => match self.db_version {
                Version::Eight => self.panel = PanelKind::Checks,
                Version::Five => self.panel = PanelKind::Options,
            },
            TAB_LEFT_KEY => {
                self.panel = PanelKind::ForeignKeys;
            }
            BACK_KEY => {
                self.handle_back_event()?;
            }
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            UP_KEY => {
                if !self.triggers.is_empty() {
                    let index = get_table_up_index(self.triggers_state.selected());
                    self.triggers_state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.triggers.is_empty() {
                    let index =
                        get_table_down_index(self.triggers_state.selected(), self.triggers.len());
                    self.triggers_state.select(Some(index));
                }
            }
            NEW_KEY => {
                self.trigger_dlg = Some(TriggerDialog::new(None));
            }
            CONFIRM_KEY => {
                if let Some(index) = self.triggers_state.selected() {
                    self.trigger_dlg = Some(TriggerDialog::new(Some(&self.triggers[index])));
                }
            }
            DELETE_KEY => {
                if self.triggers_state.selected().is_some() {
                    self.delete_trigger_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Warning,
                        "Delete Trigger",
                        "Are you sure to delete trigger?",
                    ));
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_checks_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_RIGHT_KEY => {
                self.panel = PanelKind::Options;
            }
            TAB_LEFT_KEY => {
                self.panel = PanelKind::Triggers;
            }
            BACK_KEY => {
                self.handle_back_event()?;
            }
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            UP_KEY => {
                if !self.checks.is_empty() {
                    let index = get_table_up_index(self.checks_state.selected());
                    self.checks_state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.checks.is_empty() {
                    let index =
                        get_table_down_index(self.checks_state.selected(), self.checks.len());
                    self.checks_state.select(Some(index));
                }
            }
            NEW_KEY => {
                self.check_dlg = Some(CheckDialog::new(None));
            }
            CONFIRM_KEY => {
                if let Some(index) = self.checks_state.selected() {
                    self.check_dlg = Some(CheckDialog::new(Some(&self.checks[index])));
                }
            }
            DELETE_KEY => {
                if self.triggers_state.selected().is_some() {
                    self.delete_trigger_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Warning,
                        "Delete Trigger",
                        "Are you sure to delete trigger?",
                    ));
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_options_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            TAB_LEFT_KEY => match self.db_version {
                Version::Eight => self.panel = PanelKind::Checks,
                Version::Five => self.panel = PanelKind::Triggers,
            },
            TAB_RIGHT_KEY => self.panel = PanelKind::Comment,
            _ => match self.form.handle_event(key)? {
                DialogResult::Changed(name, selected) => {
                    if name == "default character set" {
                        let collations = fetch_mysql_query(
                            self.conns.clone(),
                            self.pools.clone(),
                            self.conn_id.as_ref().unwrap(),
                            None,
                            format!("SHOW COLLATION WHERE Charset='{}'", selected).as_str(),
                        )
                        .await?;
                        self.form.set_item(
                            "default collation",
                            FormItem::new_select(
                                "default collation".to_string(),
                                collations
                                    .iter()
                                    .map(|c| c.try_get("Collation").unwrap())
                                    .collect(),
                                None,
                                true,
                                false,
                            ),
                        );
                    }
                }
                DialogResult::Cancel => {
                    self.handle_back_event()?;
                }
                _ => (),
            },
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_comment_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::Options,
            TAB_RIGHT_KEY => self.panel = PanelKind::SQLPreview,
            BACK_KEY => {
                self.handle_back_event()?;
            }
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
            _ => {
                let key: Input = key.to_owned().into();
                self.comment.input(key);
            }
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_sql_preview_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::Comment,
            TAB_RIGHT_KEY => self.panel = PanelKind::Fields,
            BACK_KEY => {
                self.handle_back_event()?;
            }
            SAVE_KEY => {
                self.handle_save_event().await?;
            }
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
                        self.table_name = Some(name.to_string());
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
    fn handle_delete_field_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.delete_field_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => self.delete_field_dlg = None,
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.fields_state.selected() {
                        self.fields.remove(index);
                    }
                    self.delete_field_dlg = None;
                    self.fields_state.select(None);
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_delete_index_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.delete_index_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_index_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.indexes_state.selected() {
                        self.indexes.remove(index);
                    }
                    self.delete_index_dlg = None;
                    self.indexes_state.select(None);
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }

    fn handle_delete_check_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.delete_check_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_check_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.checks_state.selected() {
                        self.checks.remove(index);
                    }
                    self.checks_state.select(None);
                    self.delete_check_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_delete_trigger_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.delete_trigger_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_trigger_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.triggers_state.selected() {
                        self.triggers.remove(index);
                    }
                    self.triggers_state.select(None);
                    self.delete_trigger_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_delete_foreign_key_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.delete_foreign_key_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_foreign_key_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.foreign_keys_state.selected() {
                        self.foreign_keys.remove(index);
                    }
                    self.foreign_keys_state.select(None);
                    self.delete_foreign_key_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn update_commands(&self) {
        let mut cmds = if let Some(dlg) = self.delete_field_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.delete_index_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.delete_foreign_key_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.delete_trigger_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.delete_check_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.input_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.exit_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.info_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.field_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.index_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.foreign_key_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.trigger_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.check_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(sel) = self.kind_sel.as_ref() {
            sel.get_commands()
        } else {
            self.get_main_commands()
        };

        self.cmd_bar.borrow_mut().set_commands(&mut cmds);
    }
    fn get_main_commands(&self) -> Vec<Command> {
        let mut cmds = match self.panel {
            PanelKind::Fields => self.get_field_commands(),
            PanelKind::Indexes => self.get_index_commands(),
            PanelKind::ForeignKeys => self.get_foreign_key_commands(),
            PanelKind::Triggers => self.get_trigger_commands(),
            PanelKind::Checks => self.get_check_commands(),
            PanelKind::Options => self.get_option_commands(),
            PanelKind::Comment => self.get_comment_commands(),
            PanelKind::SQLPreview => self.get_sql_preview_commands(),
        };
        cmds.push(Command {
            name: "Back",
            key: BACK_KEY,
        });
        cmds.push(Command {
            name: "Save",
            key: SAVE_KEY,
        });
        cmds
    }
    fn get_field_commands(&self) -> Vec<Command> {
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
                name: "Prev Panel",
                key: TAB_LEFT_KEY,
            },
            Command {
                name: "Add Field",
                key: NEW_KEY,
            },
        ];
        if self.fields_state.selected().is_some() {
            cmds.append(&mut vec![
                Command {
                    name: "Edit Field",
                    key: CONFIRM_KEY,
                },
                Command {
                    name: "Delete Field",
                    key: DELETE_KEY,
                },
            ]);
        }
        cmds
    }
    fn get_index_commands(&self) -> Vec<Command> {
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
                name: "Add Index",
                key: NEW_KEY,
            },
        ];
        if self.indexes_state.selected().is_some() {
            cmds.append(&mut vec![
                Command {
                    name: "Edit Index",
                    key: EDIT_KEY,
                },
                Command {
                    name: "Delete Index",
                    key: DELETE_KEY,
                },
            ]);
        }
        cmds
    }
    fn get_foreign_key_commands(&self) -> Vec<Command> {
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
                name: "Add Foreign Key",
                key: NEW_KEY,
            },
        ];
        if self.foreign_keys_state.selected().is_some() {
            cmds.append(&mut vec![
                Command {
                    name: "Edit Foreign Key",
                    key: EDIT_KEY,
                },
                Command {
                    name: "Delete Foreign Key",
                    key: DELETE_KEY,
                },
            ]);
        }
        cmds
    }
    fn get_trigger_commands(&self) -> Vec<Command> {
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
                name: "Add Trigger",
                key: NEW_KEY,
            },
        ];
        if self.triggers_state.selected().is_some() {
            cmds.append(&mut vec![
                Command {
                    name: "Edit Trigger",
                    key: EDIT_KEY,
                },
                Command {
                    name: "Delete Trigger",
                    key: DELETE_KEY,
                },
            ]);
        }
        cmds
    }
    fn get_check_commands(&self) -> Vec<Command> {
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
                name: "Add Check",
                key: NEW_KEY,
            },
        ];
        if self.checks_state.selected().is_some() {
            cmds.append(&mut vec![
                Command {
                    name: "Edit Check",
                    key: EDIT_KEY,
                },
                Command {
                    name: "Delete Check",
                    key: DELETE_KEY,
                },
            ]);
        }
        cmds
    }
    fn get_option_commands(&self) -> Vec<Command> {
        let mut cmds = vec![
            Command {
                name: "Next Panel",
                key: TAB_RIGHT_KEY,
            },
            Command {
                name: "Previous Panel",
                key: TAB_LEFT_KEY,
            },
        ];
        cmds.extend(self.form.get_commands());
        cmds
    }
    fn get_comment_commands(&self) -> Vec<Command> {
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
    fn get_sql_preview_commands(&self) -> Vec<Command> {
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
