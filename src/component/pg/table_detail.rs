use crate::{
    app::{ComponentResult, DialogResult, MainPanel},
    component::{get_table_down_index, get_table_up_index, Command, CommandBarComponent},
    dialog::{
        confirm::{ConfirmDialog, Kind as ConfirmKind},
        pg::{
            CheckDialog, ExcludeDialog, FieldDialog, ForeignKeyDialog, IndexDialog, RuleDialog,
            TriggerDialog, UniqueDialog,
        },
        InputDialog,
    },
    event::{config::*, Key},
    model::pg::{
        convert_row_to_pg_check, convert_row_to_pg_exclude, convert_row_to_pg_rule,
        convert_row_to_pg_trigger, convert_show_column_to_pg_fields, convert_show_fk_to_pg_fk,
        convert_show_index_to_pg_indexes, convert_show_unique_to_pg_unique, get_all_pg_schemas,
        get_pg_field_names, get_pg_schemas, get_pg_table_names, Check, Connections, DoInstead,
        EventKind, Exclude, ExcludeElement, Field, FieldKind, FiresKind, ForEachKind, ForeignKey,
        Index, IndexField, IndexMethod, OnDeleteKind, OnUpdateKind, Rule, Trigger, Unique,
    },
    pool::{execute_pg_query_unprepared, fetch_one_pg, fetch_pg_query, get_pg_pool, PGPools},
};
use anyhow::Result;
use sqlx::Row;
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

pub enum PanelKind {
    Fields,
    Indexes,
    ForeignKeys,
    Uniques,
    Checks,
    Excludes,
    Rules,
    Triggers,
    Comment,
    SQLPreview,
}

pub struct TableDetailComponent<'a> {
    conn_id: Option<Uuid>,
    db_name: Option<String>,
    schema_name: Option<String>,
    table_name: Option<String>,
    key_name: Option<String>,
    panel: PanelKind,
    fields: Vec<Field>,
    old_fields: Vec<Field>,
    indexes: Vec<Index>,
    old_indexes: Vec<Index>,
    foreign_keys: Vec<ForeignKey>,
    old_foreign_keys: Vec<ForeignKey>,
    uniques: Vec<Unique>,
    old_uniques: Vec<Unique>,
    checks: Vec<Check>,
    old_checks: Vec<Check>,
    excludes: Vec<Exclude>,
    old_excludes: Vec<Exclude>,
    rules: Vec<Rule>,
    old_rules: Vec<Rule>,
    triggers: Vec<Trigger>,
    old_triggers: Vec<Trigger>,
    comment: TextArea<'a>,
    old_comment: TextArea<'a>,
    sql_preview: TextArea<'a>,
    fields_state: TableState,
    indexes_state: TableState,
    foreign_keys_state: TableState,
    uniques_state: TableState,
    checks_state: TableState,
    excludes_state: TableState,
    rules_state: TableState,
    triggers_state: TableState,
    input_dlg: Option<InputDialog<'a>>,
    exit_dlg: Option<ConfirmDialog>,
    info_dlg: Option<ConfirmDialog>,
    delete_field_dlg: Option<ConfirmDialog>,
    delete_index_dlg: Option<ConfirmDialog>,
    delete_foreign_key_dlg: Option<ConfirmDialog>,
    delete_unique_dlg: Option<ConfirmDialog>,
    delete_check_dlg: Option<ConfirmDialog>,
    delete_exclude_dlg: Option<ConfirmDialog>,
    delete_rule_dlg: Option<ConfirmDialog>,
    delete_trigger_dlg: Option<ConfirmDialog>,
    field_dlg: Option<FieldDialog<'a>>,
    index_dlg: Option<IndexDialog<'a>>,
    foreign_key_dlg: Option<ForeignKeyDialog<'a>>,
    unique_dlg: Option<UniqueDialog<'a>>,
    exclude_dlg: Option<ExcludeDialog<'a>>,
    rule_dlg: Option<RuleDialog<'a>>,
    check_dlg: Option<CheckDialog<'a>>,
    trigger_dlg: Option<TriggerDialog<'a>>,
    cmd_bar: Rc<RefCell<CommandBarComponent>>,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<PGPools>>,
}

impl<'a> TableDetailComponent<'a> {
    pub fn new(
        cmd_bar: Rc<RefCell<CommandBarComponent>>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<PGPools>>,
    ) -> Self {
        TableDetailComponent {
            table_name: None,
            panel: PanelKind::Fields,
            fields: Vec::new(),
            old_fields: Vec::new(),
            indexes: Vec::new(),
            old_indexes: Vec::new(),
            uniques: Vec::new(),
            old_uniques: Vec::new(),
            checks: Vec::new(),
            old_checks: Vec::new(),
            foreign_keys: Vec::new(),
            old_foreign_keys: Vec::new(),
            excludes: Vec::new(),
            old_excludes: Vec::new(),
            rules: Vec::new(),
            old_rules: Vec::new(),
            triggers: Vec::new(),
            old_triggers: Vec::new(),
            comment: TextArea::default(),
            old_comment: TextArea::default(),
            sql_preview: TextArea::default(),
            conn_id: None,
            db_name: None,
            schema_name: None,
            key_name: None,
            input_dlg: None,
            exit_dlg: None,
            info_dlg: None,
            delete_field_dlg: None,
            delete_index_dlg: None,
            delete_unique_dlg: None,
            delete_exclude_dlg: None,
            delete_rule_dlg: None,
            delete_foreign_key_dlg: None,
            delete_trigger_dlg: None,
            delete_check_dlg: None,
            fields_state: TableState::default(),
            indexes_state: TableState::default(),
            foreign_keys_state: TableState::default(),
            uniques_state: TableState::default(),
            excludes_state: TableState::default(),
            rules_state: TableState::default(),
            triggers_state: TableState::default(),
            checks_state: TableState::default(),
            field_dlg: None,
            index_dlg: None,
            foreign_key_dlg: None,
            unique_dlg: None,
            exclude_dlg: None,
            rule_dlg: None,
            trigger_dlg: None,
            check_dlg: None,
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
        table_name: Option<&str>,
    ) -> Result<()> {
        self.conn_id = Some(*conn_id);
        self.db_name = Some(db_name.to_string());
        self.schema_name = Some(schema_name.to_string());
        self.table_name = table_name.map(|s| s.to_string());

        if let Some(table_name) = &self.table_name.as_ref() {
            let fields = fetch_pg_query(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!(
                    "SELECT col_description((table_schema||'.'||table_name)::regclass::oid, ordinal_position) as comment,* FROM information_schema.columns WHERE table_schema = '{}' and table_name = '{}' order by ordinal_position ASC",
                    schema_name, table_name
                ),
            )
            .await?;
            let keys = fetch_pg_query(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!("SELECT a.attname FROM pg_index i JOIN pg_attribute a ON a.attrelid = i.indrelid and a.attnum = ANY(i.indkey) WHERE i.indrelid = '{}'::regclass AND i.indisprimary", table_name),
            )
            .await?;
            self.fields = convert_show_column_to_pg_fields(
                fields,
                keys.iter()
                    .map(|k| k.try_get::<String, _>("attname").unwrap())
                    .collect::<Vec<String>>(),
            );
            self.old_fields = self.fields.clone();

            let pr_key = fetch_one_pg(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!("select conname from pg_constraint where conrelid = '{}'::regclass and contype='p'",table_name)
            ).await?;
            self.key_name = pr_key.map(|key| key.try_get("conname").unwrap());

            let indexes = fetch_pg_query(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!(
                    "SELECT obj_description(indexname::regclass) as comment, inds.* FROM pg_indexes AS inds JOIN pg_index AS ind ON inds.indexname::regclass = ind.indexrelid WHERE inds.tablename='{}' AND inds.schemaname='{}' AND ind.indisprimary = false",
                    table_name, schema_name
                ),
            )
            .await?;
            self.indexes = convert_show_index_to_pg_indexes(indexes);
            self.old_indexes = self.indexes.clone();

            let foreign_keys = fetch_pg_query(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!(
                    r"
                    SELECT 
                        obj_description(oid) as comment,
                        conrelid::regclass AS table_name,
                        conname AS foreign_key,
                        pg_get_constraintdef(oid) AS def
                    FROM pg_constraint
                    WHERE contype = 'f' and conrelid::regclass::text = '{}'
                    AND connamespace = '{}'::regnamespace
                    ORDER BY conrelid::regclass::text, contype DESC
                    ",
                    table_name, schema_name,
                ),
            )
            .await?;

            self.foreign_keys = convert_show_fk_to_pg_fk(schema_name, foreign_keys);
            self.old_foreign_keys = self.foreign_keys.clone();

            let uniques = fetch_pg_query(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!(
                    r"
                    SELECT 
                        obj_description(c.oid) as comment,
                        c.conname AS constraint_name,
                        array_agg(a.attname ORDER BY k.n) AS columns
                    FROM pg_constraint AS c
                    CROSS JOIN LATERAL unnest(c.conkey) WITH ORDINALITY AS k(c,n)
                    JOIN pg_attribute AS a
                    ON a.attnum = k.c AND a.attrelid = c.conrelid
                    WHERE c.contype = 'u'
                    AND c.connamespace  = '{}'::regnamespace
                    AND c.conrelid = '{}'::regclass
                    GROUP BY c.oid, c.conrelid, c.conname
                    ",
                    schema_name, table_name,
                ),
            )
            .await?;
            self.uniques = convert_show_unique_to_pg_unique(uniques);
            self.old_uniques = self.uniques.clone();
            let checks = fetch_pg_query(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!(
                    r"
                    SELECT
                        obj_description(pgc.oid) as comment,
                        pgc.conname AS constraint_name,
                        pg_get_constraintdef(pgc.oid) AS def
                    FROM pg_constraint pgc
                    JOIN pg_namespace nsp ON nsp.oid = pgc.connamespace
                    JOIN pg_class cls ON pgc.conrelid = cls.oid
                    LEFT JOIN information_schema.constraint_column_usage ccu
                    ON pgc.conname = ccu.constraint_name
                    AND nsp.nspname = ccu.constraint_schema
                    WHERE contype = 'c'
                    AND ccu.table_schema = '{}'
                    AND ccu.table_name = '{}'
                    ORDER BY pgc.conname
                ",
                    schema_name, table_name
                ),
            )
            .await?;
            self.checks = convert_row_to_pg_check(checks);
            self.old_checks = self.checks.clone();

            let excludes = fetch_pg_query(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!(
                    r"
                    SELECT
                        obj_description(oid) as comment,
                        pg_catalog.pg_get_constraintdef(oid,true) AS def,
                        conname
                    FROM pg_constraint
                    WHERE contype = 'x'
                    AND conrelid = '{}'::regclass
                    AND connamespace = '{}'::regnamespace
                    ",
                    table_name, schema_name
                ),
            )
            .await?;
            self.excludes = convert_row_to_pg_exclude(excludes);
            self.old_excludes = self.excludes.clone();

            let rules = fetch_pg_query(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!(
                    r"
                    SELECT
                        *
                    FROM
                        pg_rules
                    WHERE schemaname='{}' and tablename='{}'",
                    schema_name, table_name,
                ),
            )
            .await?;

            self.rules = convert_row_to_pg_rule(rules);
            self.old_rules = self.rules.clone();

            let triggers = fetch_pg_query(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!(
                    r"
                    SELECT
                        tgname,
	                    proname,
	                    tgtype,
	                    tgenabled,
	                    nspname,
	                    tgargs,
	                    tgqual,
	                    array_agg(attname) AS columns
                    FROM
	                    pg_trigger
	                JOIN pg_proc ON pg_proc.OID = tgfoid
	                JOIN pg_namespace ON pg_namespace.OID = pronamespace 
	                LEFT JOIN pg_attribute ON pg_attribute.attrelid = tgrelid and pg_attribute.attnum = ANY(tgattr)
                    WHERE
	                    tgrelid = '{}'::regclass
	                GROUP BY
                        tgname, proname, tgtype, tgenabled, nspname, tgargs, tgqual",
                    table_name,
                ),
            )
            .await?;

            self.triggers = convert_row_to_pg_trigger(&triggers);
            self.old_triggers = self.triggers.clone();

            self.comment = if let Some(c) = fetch_one_pg(
                self.conns.clone(),
                self.pools.clone(),
                conn_id,
                Some(db_name),
                &format!(
                    "SELECT obj_description('{}'::regclass) as comment",
                    table_name
                ),
            )
            .await?
            .unwrap()
            .try_get::<Option<&str>, _>("comment")
            .unwrap()
            {
                TextArea::from(c.lines())
            } else {
                TextArea::default()
            };

            self.old_comment = self.comment.clone();
        }

        Ok(())
    }

    pub fn draw<B>(&mut self, f: &mut Frame<B>, r: Rect, is_focus: bool)
    where
        B: Backend,
    {
        f.render_widget(
            Block::default()
                .title(if let Some(name) = &self.table_name {
                    format!("Edit {name}")
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
        let select_tab = match self.panel {
            PanelKind::Fields => 0,
            PanelKind::Indexes => 1,
            PanelKind::ForeignKeys => 2,
            PanelKind::Uniques => 3,
            PanelKind::Checks => 4,
            PanelKind::Excludes => 5,
            PanelKind::Rules => 6,
            PanelKind::Triggers => 7,
            PanelKind::Comment => 8,
            PanelKind::SQLPreview => 9,
        };
        f.render_widget(
            Tabs::new(
                [
                    Span::raw("Fields"),
                    Span::raw("Indexes"),
                    Span::raw("Foreign Keys"),
                    Span::raw("Uniques"),
                    Span::raw("Checks"),
                    Span::raw("Excludes"),
                    Span::raw("Rules"),
                    Span::raw("Triggers"),
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
            .select(select_tab),
            chunks[0],
        );
        match self.panel {
            PanelKind::Fields => self.draw_fields(f, chunks[1]),
            PanelKind::Indexes => self.draw_indexes(f, chunks[1]),
            PanelKind::ForeignKeys => self.draw_foreign_keys(f, chunks[1]),
            PanelKind::Uniques => self.draw_uniques(f, chunks[1]),
            PanelKind::Checks => self.draw_checks(f, chunks[1]),
            PanelKind::Excludes => self.draw_excludes(f, chunks[1]),
            PanelKind::Rules => self.draw_rules(f, chunks[1]),
            PanelKind::Triggers => self.draw_triggers(f, chunks[1]),
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
        if let Some(dlg) = self.unique_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.exclude_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.rule_dlg.as_mut() {
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
        if let Some(dlg) = self.delete_unique_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.delete_exclude_dlg.as_mut() {
            dlg.draw(f);
        }
        if let Some(dlg) = self.delete_rule_dlg.as_mut() {
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
        let result = if self.input_dlg.is_some() {
            self.handle_input_dlg_event(key).await?
        } else if self.exit_dlg.is_some() {
            self.handle_exit_event(key)
        } else if self.info_dlg.is_some() {
            self.handle_info_event(key)
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
        } else if self.delete_exclude_dlg.is_some() {
            self.handle_delete_exclude_event(key)
        } else if self.delete_unique_dlg.is_some() {
            self.handle_delete_unique_event(key)
        } else if self.delete_rule_dlg.is_some() {
            self.handle_delete_rule_event(key)
        } else if self.field_dlg.is_some() {
            self.handle_field_dlg_event(key)?
        } else if self.index_dlg.is_some() {
            self.handle_index_dlg_event(key).await?
        } else if self.foreign_key_dlg.is_some() {
            self.handle_foreign_key_dlg_event(key).await?
        } else if self.unique_dlg.is_some() {
            self.handle_unique_dlg_event(key)?
        } else if self.rule_dlg.is_some() {
            self.handle_rule_dlg_event(key)?
        } else if self.exclude_dlg.is_some() {
            self.handle_exclude_dlg_event(key).await?
        } else if self.trigger_dlg.is_some() {
            self.handle_trigger_dlg_event(key)?
        } else if self.check_dlg.is_some() {
            self.handle_check_dlg_event(key)?
        } else {
            self.handle_main_event(key).await?
        };
        Ok(result)
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
                        f.name().to_string(),
                        f.kind().to_string(),
                        f.length().unwrap_or_default(),
                        f.default_value().map(|s| s.to_string()).unwrap_or_default(),
                        if f.not_null() { "\u{2705}" } else { "\u{274E}" }.to_string(),
                        if f.key() { "\u{2705}" } else { "\u{274E}" }.to_string(),
                        f.comment().map(|s| s.to_string()).unwrap_or_default(),
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec![
            "Name", "Type", "Length", "Default", "Not Null", "Key", "Comment",
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

    fn draw_uniques<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(
            self.uniques
                .iter()
                .map(|u| {
                    RowUI::new(vec![
                        u.name().to_string(),
                        u.fields().join(","),
                        u.comment().map(|c| c.to_string()).unwrap_or_default(),
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec!["Name", "Fields", "Comment"]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(table, r, &mut self.uniques_state);
    }
    fn draw_excludes<B>(&mut self, f: &mut Frame<B>, r: Rect)
    where
        B: Backend,
    {
        let table = Table::new(
            self.excludes
                .iter()
                .map(|e| {
                    RowUI::new(vec![
                        e.name().to_string(),
                        e.index_method().unwrap_or("").to_string(),
                        e.element()
                            .iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<String>>()
                            .join(","),
                        e.comment().map(|s| s.to_string()).unwrap_or_default(),
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec![
            "Name",
            "Index Method",
            "Element",
            "Comment",
        ]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(table, r, &mut self.excludes_state);
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
                        if r.enable() { "true" } else { "false" },
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
            "Enable",
            "Where",
            "Definition",
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
        f.render_stateful_widget(table, r, &mut self.rules_state);
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
            "new_table"
        };
        let mut table_ddl = Vec::new();
        let mut fields_ddl = Vec::new();
        let mut comments_ddl = Vec::new();

        self.fields.iter().for_each(|f| {
            let (field_ddl, comment_ddl) =
                f.get_create_ddl(self.schema_name.as_deref().unwrap(), table_name);
            fields_ddl.push(field_ddl);
            if let Some(c) = comment_ddl {
                comments_ddl.push(c);
            }
        });
        table_ddl.append(&mut fields_ddl);

        let key_fields: Vec<&Field> = self.fields.iter().filter(|f| f.key()).collect();
        if !key_fields.is_empty() {
            let names: Vec<String> = key_fields
                .iter()
                .map(|f| format!(r#""{}""#, f.name()))
                .collect();
            table_ddl.push(format!("\nPRIMARY KEY ({})", names.join(",")));
        }
        self.foreign_keys.iter().for_each(|fk| {
            let (fk_ddl, comment_ddl) = fk.get_create_ddl(self.schema_name().unwrap(), table_name);
            table_ddl.push(fk_ddl);
            if let Some(c) = comment_ddl {
                comments_ddl.push(c);
            }
        });

        self.uniques.iter().for_each(|unique| {
            let (un_ddl, comment_ddl) =
                unique.get_create_ddl(self.schema_name().unwrap(), table_name);
            table_ddl.push(un_ddl);
            if let Some(c) = comment_ddl {
                comments_ddl.push(c);
            }
        });

        self.checks.iter().for_each(|check| {
            let (check_ddl, comment_ddl) =
                check.get_create_ddl(self.schema_name().unwrap(), table_name);
            table_ddl.push(check_ddl);
            if let Some(c) = comment_ddl {
                comments_ddl.push(c);
            }
        });

        self.excludes.iter().for_each(|exclude| {
            let (exclude_ddl, comment_ddl) =
                exclude.get_create_ddl(self.schema_name().unwrap(), table_name);
            table_ddl.push(exclude_ddl);
            if let Some(c) = comment_ddl {
                comments_ddl.push(c);
            }
        });

        let rule_sqls: Vec<String> = if !self.rules.is_empty() {
            self.rules
                .iter()
                .map(|rule| {
                    rule.get_create_ddl(
                        self.schema_name.as_deref().unwrap(),
                        self.table_name.as_deref().unwrap(),
                    )
                })
                .collect()
        } else {
            vec![]
        };

        let trigger_sqls = if !self.triggers.is_empty() {
            self.triggers
                .iter()
                .map(|t| t.get_create_ddl(self.schema_name.as_deref().unwrap(), table_name))
                .collect()
        } else {
            vec![]
        };
        let mut indexes_ddl = Vec::new();
        self.indexes.iter().for_each(|index| {
            let (index_ddl, comment_ddl) =
                index.get_create_ddl(self.schema_name.as_deref().unwrap(), table_name);
            indexes_ddl.push(index_ddl);
            if let Some(c) = comment_ddl {
                comments_ddl.push(c);
            }
        });

        format!(
            "CREATE TABLE \"{}\".\"{}\" (\n{}\n);
            {}
            {}
            {}
            {}",
            self.schema_name.as_deref().unwrap(),
            table_name,
            table_ddl.join(",\n"),
            indexes_ddl.join("\n"),
            rule_sqls.join("\n"),
            trigger_sqls.join("\n"),
            comments_ddl.join("\n")
        )
    }
    fn build_alter_ddl(&self) -> String {
        let mut ddl: Vec<String> = Vec::new();
        let mut main_table_ddl: Vec<String> = Vec::new();

        let (mut rename_field_ddl, mut field_comments_ddl, mut alter_field_ddl) =
            self.build_field_alter_ddl();
        main_table_ddl.append(&mut alter_field_ddl);

        let old_key_fields: Vec<&str> = self
            .old_fields
            .iter()
            .filter(|f| f.key())
            .map(|f| f.name())
            .collect();
        let key_fields: Vec<&str> = self
            .fields
            .iter()
            .filter(|f| f.key())
            .map(|f| f.name())
            .collect();
        let match_len = key_fields
            .iter()
            .zip(&old_key_fields)
            .filter(|(a, b)| a == b)
            .count();
        if key_fields.len() != old_key_fields.len() || match_len != key_fields.len() {
            if let Some(kname) = self.key_name.as_ref() {
                main_table_ddl.push(format!(r#"DROP CONSTRAINT "{}""#, kname,));
            }
            main_table_ddl.push(format!("ADD PRIMARY KEY ({})", key_fields.join(",")));
        }

        let (mut rename_index_ddl, mut index_comments_ddl, mut alter_index_ddl) =
            self.build_index_alter_ddl();
        let (mut rename_fk_ddl, mut fk_comments_ddl, mut alter_fk_ddl) =
            self.build_foreign_key_alter_ddl();
        main_table_ddl.append(&mut alter_fk_ddl);
        let (mut rename_unique_ddl, mut unique_comments_ddl, mut alter_unique_ddl) =
            self.build_unique_alter_ddl();
        main_table_ddl.append(&mut alter_unique_ddl);
        let (mut rename_check_ddl, mut check_comments_ddl, mut alter_check_ddl) =
            self.build_check_alter_ddl();
        main_table_ddl.append(&mut alter_check_ddl);

        let (mut rename_exclude_ddl, mut exclude_comments_ddl, mut alter_exclude_ddl) =
            self.build_exclude_alter_ddl();
        main_table_ddl.append(&mut alter_exclude_ddl);
        let (mut alter_rule_ddl, mut rule_comments_ddl) = self.build_rule_alter_ddl();
        let (mut alter_trigger_ddl, mut trigger_comments_ddl) = self.build_trigger_alter_ddl();

        ddl.append(&mut rename_field_ddl);
        ddl.append(&mut rename_index_ddl);
        ddl.append(&mut rename_fk_ddl);
        ddl.append(&mut rename_unique_ddl);
        ddl.append(&mut rename_check_ddl);
        ddl.append(&mut rename_exclude_ddl);
        ddl.append(&mut alter_rule_ddl);
        ddl.append(&mut alter_trigger_ddl);

        if !main_table_ddl.is_empty() {
            ddl.push(format!(
                r#"ALTER TABLE "{}"."{}""#,
                self.schema_name.as_deref().unwrap(),
                self.table_name.as_deref().unwrap()
            ));
            ddl.push(format!("{};", main_table_ddl.join(",\n")));
        }
        ddl.append(&mut alter_index_ddl);
        ddl.append(&mut field_comments_ddl);
        ddl.append(&mut index_comments_ddl);
        ddl.append(&mut fk_comments_ddl);
        ddl.append(&mut unique_comments_ddl);
        ddl.append(&mut check_comments_ddl);
        ddl.append(&mut exclude_comments_ddl);
        ddl.append(&mut rule_comments_ddl);
        ddl.append(&mut trigger_comments_ddl);

        if self.comment.lines() != self.old_comment.lines() {
            ddl.push(format!(
                r#"COMMENT ON TABLE "{}"."{}" IS '{}';"#,
                self.schema_name.as_deref().unwrap(),
                self.table_name.as_deref().unwrap(),
                self.comment.lines().join("\n")
            ));
        }
        ddl.join("\n")
    }
    fn build_field_alter_ddl(&self) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut alter_table_ddl = Vec::new();
        let mut rename_ddl = Vec::new();
        let mut comments_ddl = Vec::new();

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
        let mut drop_fields_ddl = self
            .old_fields
            .iter()
            .filter(|field| !field_ids.contains(field.id()))
            .map(|field| field.get_drop_ddl())
            .collect();
        alter_table_ddl.append(&mut drop_fields_ddl);
        self.fields.iter().for_each(|field| {
            if !old_field_ids.contains(field.id()) {
                let (field_ddl, comment_ddl) = field.get_add_ddl(
                    self.schema_name.as_deref().unwrap(),
                    self.table_name.as_deref().unwrap(),
                );

                alter_table_ddl.push(field_ddl);
                if let Some(comment) = comment_ddl {
                    comments_ddl.push(comment);
                }
            } else {
                let same_field = self
                    .old_fields
                    .iter()
                    .find(|f| f.id() == field.id())
                    .unwrap();

                let mut rename_field = field.get_rename_ddl(
                    self.schema_name.as_deref().unwrap(),
                    self.table_name.as_ref().unwrap(),
                    same_field,
                );
                if !rename_field.is_empty() {
                    rename_ddl.append(&mut rename_field);
                }

                let (mut alter_field, comment_ddl) = field.get_alter_ddl(
                    same_field,
                    self.schema_name.as_deref().unwrap(),
                    self.table_name.as_deref().unwrap(),
                );
                if !alter_field.is_empty() {
                    alter_table_ddl.append(&mut alter_field);
                }
                if let Some(comment) = comment_ddl {
                    comments_ddl.push(comment);
                }
            }
        });
        (rename_ddl, comments_ddl, alter_table_ddl)
    }
    fn build_index_alter_ddl(&self) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut ddl = Vec::new();
        let mut rename_ddl = Vec::new();
        let mut comments_ddl = Vec::new();

        let index_ids = self
            .indexes
            .iter()
            .map(|i| i.id().to_owned())
            .collect::<Vec<Uuid>>();
        let old_index_ids = self
            .old_indexes
            .iter()
            .map(|i| i.id().to_owned())
            .collect::<Vec<Uuid>>();
        let mut drop_index_ddl: Vec<String> = self
            .old_indexes
            .iter()
            .filter(|index| !index_ids.contains(index.id()))
            .map(|index| index.get_drop_ddl())
            .collect();

        ddl.append(&mut drop_index_ddl);
        self.indexes.iter().for_each(|index| {
            if !old_index_ids.contains(index.id()) {
                let (index_ddl, comment_ddl) = index.get_add_ddl(
                    self.schema_name.as_deref().unwrap(),
                    self.table_name.as_deref().unwrap(),
                );
                ddl.push(index_ddl);
                if let Some(comment) = comment_ddl {
                    comments_ddl.push(comment);
                }
            } else {
                let same_index = self
                    .old_indexes
                    .iter()
                    .find(|i| i.id() == index.id())
                    .unwrap();
                let mut rename_index = index.get_rename_ddl(same_index);
                if !rename_index.is_empty() {
                    rename_ddl.append(&mut rename_index);
                }

                let (mut alter_ddl, comment_ddl) =
                    index.get_alter_ddl(same_index, self.schema_name.as_deref().unwrap());
                if !alter_ddl.is_empty() {
                    ddl.append(&mut alter_ddl);
                }
                if let Some(comment) = comment_ddl {
                    comments_ddl.push(comment);
                }
            }
        });
        (rename_ddl, comments_ddl, ddl)
    }
    fn build_foreign_key_alter_ddl(&self) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut alter_table_ddl = Vec::new();
        let mut rename_ddl = Vec::new();
        let mut comments_ddl = Vec::new();
        let schema_name = self.schema_name().unwrap();
        let table_name = self.table_name().unwrap();

        let fk_ids = self
            .foreign_keys
            .iter()
            .map(|fk| fk.id().to_owned())
            .collect::<Vec<Uuid>>();
        let old_fk_ids = self
            .old_foreign_keys
            .iter()
            .map(|fk| fk.id().to_owned())
            .collect::<Vec<Uuid>>();
        let mut drop_fks_str = self
            .old_foreign_keys
            .iter()
            .filter(|fk| !fk_ids.contains(fk.id()))
            .map(|fk| fk.get_drop_ddl())
            .collect();

        alter_table_ddl.append(&mut drop_fks_str);
        self.foreign_keys.iter().for_each(|fk| {
            if !old_fk_ids.contains(fk.id()) {
                let (fk_ddl, comment_ddl) = fk.get_add_ddl(schema_name, table_name);
                alter_table_ddl.push(fk_ddl);
                if let Some(comment) = comment_ddl {
                    comments_ddl.push(comment);
                }
            } else {
                let same_fk = self
                    .old_foreign_keys
                    .iter()
                    .find(|f| f.id() == fk.id())
                    .unwrap();

                let mut rename_fk = fk.get_rename_ddl(same_fk, self.table_name.as_ref().unwrap());
                if !rename_fk.is_empty() {
                    rename_ddl.append(&mut rename_fk);
                }

                let (mut alter_fk, comment_ddl) =
                    fk.get_alter_ddl(same_fk, schema_name, table_name);
                if !alter_fk.is_empty() {
                    alter_table_ddl.append(&mut alter_fk);
                }
                if let Some(comment) = comment_ddl {
                    comments_ddl.push(comment);
                }
            }
        });
        (rename_ddl, comments_ddl, alter_table_ddl)
    }
    fn build_exclude_alter_ddl(&self) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut alter_table_ddl = Vec::new();
        let mut rename_ddl = Vec::new();
        let mut comments_ddl = Vec::new();

        let exclude_ids = self
            .excludes
            .iter()
            .map(|exclude| exclude.id().to_owned())
            .collect::<Vec<Uuid>>();
        let old_exclude_ids = self
            .old_excludes
            .iter()
            .map(|exclude| exclude.id().to_owned())
            .collect::<Vec<Uuid>>();
        let mut drop_excludes_str = self
            .old_excludes
            .iter()
            .filter(|exclude| !exclude_ids.contains(exclude.id()))
            .map(|exclude| exclude.get_drop_ddl())
            .collect();
        alter_table_ddl.append(&mut drop_excludes_str);
        self.excludes.iter().for_each(|exclude| {
            if !old_exclude_ids.contains(exclude.id()) {
                let (exclude_ddl, comment_ddl) =
                    exclude.get_add_ddl(self.schema_name().unwrap(), self.table_name().unwrap());
                alter_table_ddl.push(exclude_ddl);
                if let Some(comment) = comment_ddl {
                    comments_ddl.push(comment);
                }
            } else {
                let same_exclude = self
                    .old_excludes
                    .iter()
                    .find(|ex| ex.id() == exclude.id())
                    .unwrap();

                let mut rename_exclude =
                    exclude.get_rename_ddl(same_exclude, self.table_name.as_ref().unwrap());
                if !rename_exclude.is_empty() {
                    rename_ddl.append(&mut rename_exclude);
                }

                let (mut alter_exclude, comment_ddl) = exclude.get_alter_ddl(
                    same_exclude,
                    self.schema_name.as_deref().unwrap(),
                    self.table_name.as_deref().unwrap(),
                );
                if !alter_exclude.is_empty() {
                    alter_table_ddl.append(&mut alter_exclude);
                }
                if let Some(comment) = comment_ddl {
                    comments_ddl.push(comment);
                }
            }
        });
        (rename_ddl, comments_ddl, alter_table_ddl)
    }
    fn build_rule_alter_ddl(&self) -> (Vec<String>, Vec<String>) {
        let mut ddl = Vec::new();
        let mut comments_ddl = Vec::new();

        let rule_ids = self
            .rules
            .iter()
            .map(|r| r.id().to_owned())
            .collect::<Vec<Uuid>>();
        let old_rule_ids = self
            .old_rules
            .iter()
            .map(|r| r.id().to_owned())
            .collect::<Vec<Uuid>>();
        let mut drop_rule_ddl: Vec<String> = self
            .old_rules
            .iter()
            .filter(|r| !rule_ids.contains(r.id()))
            .map(|r| r.get_drop_ddl(self.table_name.as_deref().unwrap()))
            .collect();

        ddl.append(&mut drop_rule_ddl);
        self.rules.iter().for_each(|rule| {
            if !old_rule_ids.contains(rule.id()) {
                ddl.push(rule.get_add_ddl(
                    self.schema_name.as_deref().unwrap(),
                    self.table_name.as_deref().unwrap(),
                ));
            } else {
                let same_rule = self.old_rules.iter().find(|r| r.id() == rule.id()).unwrap();

                let (mut alter_ddl, comment_ddl) = rule.get_alter_ddl(
                    same_rule,
                    self.schema_name.as_deref().unwrap(),
                    self.table_name.as_deref().unwrap(),
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
    fn build_unique_alter_ddl(&self) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut alter_table_ddl = Vec::new();
        let mut rename_ddl = Vec::new();
        let mut comments_ddl = Vec::new();
        let schema_name = self.schema_name().unwrap();
        let table_name = self.table_name().unwrap();

        let unique_ids = self
            .uniques
            .iter()
            .map(|unique| unique.id().to_owned())
            .collect::<Vec<Uuid>>();
        let old_unique_ids = self
            .old_uniques
            .iter()
            .map(|unique| unique.id().to_owned())
            .collect::<Vec<Uuid>>();
        let mut drop_uniques_ddl = self
            .old_uniques
            .iter()
            .filter(|unique| !unique_ids.contains(unique.id()))
            .map(|unique| unique.get_drop_ddl())
            .collect();

        alter_table_ddl.append(&mut drop_uniques_ddl);
        self.uniques.iter().for_each(|unique| {
            if !old_unique_ids.contains(unique.id()) {
                let (un_ddl, comment_ddl) = unique.get_add_ddl(schema_name, table_name);
                alter_table_ddl.push(un_ddl);
                if let Some(comment) = comment_ddl {
                    comments_ddl.push(comment);
                }
            } else {
                let same_unique = self
                    .old_uniques
                    .iter()
                    .find(|u| u.id() == unique.id())
                    .unwrap();

                let mut rename_unique =
                    unique.get_rename_ddl(same_unique, self.table_name.as_ref().unwrap());
                if !rename_unique.is_empty() {
                    rename_ddl.append(&mut rename_unique);
                }

                let (mut alter_unique_ddl, comment_ddl) =
                    unique.get_alter_ddl(same_unique, schema_name, table_name);
                if !alter_unique_ddl.is_empty() {
                    alter_table_ddl.append(&mut alter_unique_ddl);
                }
                if let Some(comment) = comment_ddl {
                    comments_ddl.push(comment);
                }
            }
        });
        (rename_ddl, comments_ddl, alter_table_ddl)
    }
    fn build_trigger_alter_ddl(&self) -> (Vec<String>, Vec<String>) {
        let mut ddl = Vec::new();
        let mut comments_ddl = Vec::new();
        let trigger_ids = self
            .triggers
            .iter()
            .map(|t| t.id().to_owned())
            .collect::<Vec<Uuid>>();
        let old_trigger_ids = self
            .old_triggers
            .iter()
            .map(|t| t.id().to_owned())
            .collect::<Vec<Uuid>>();
        let mut drop_trigger_ddl: Vec<String> = self
            .old_triggers
            .iter()
            .filter(|t| !trigger_ids.contains(t.id()))
            .map(|t| t.get_drop_ddl(self.table_name.as_deref().unwrap()))
            .collect();
        ddl.append(&mut drop_trigger_ddl);
        self.triggers.iter().for_each(|trigger| {
            if !old_trigger_ids.contains(trigger.id()) {
                ddl.push(trigger.get_add_ddl(
                    self.schema_name.as_deref().unwrap(),
                    self.table_name.as_deref().unwrap(),
                ));
            } else {
                let same_trigger = self
                    .old_triggers
                    .iter()
                    .find(|t| t.id() == trigger.id())
                    .unwrap();

                let (mut alter_ddl, comment_ddl) = trigger.get_alter_ddl(
                    same_trigger,
                    self.schema_name.as_deref().unwrap(),
                    self.table_name.as_deref().unwrap(),
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
    fn build_check_alter_ddl(&self) -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut alter_table_ddl = Vec::new();
        let mut rename_ddl = Vec::new();
        let mut comments_ddl = Vec::new();

        let check_ids = self
            .checks
            .iter()
            .map(|check| check.id().to_owned())
            .collect::<Vec<Uuid>>();
        let old_check_ids = self
            .old_checks
            .iter()
            .map(|check| check.id().to_owned())
            .collect::<Vec<Uuid>>();
        let mut drop_checks_ddl = self
            .old_checks
            .iter()
            .filter(|check| !check_ids.contains(check.id()))
            .map(|check| check.get_drop_ddl())
            .collect();
        alter_table_ddl.append(&mut drop_checks_ddl);
        self.checks.iter().for_each(|check| {
            if !old_check_ids.contains(check.id()) {
                let (check_ddl, comment_ddl) =
                    check.get_add_ddl(self.schema_name().unwrap(), self.table_name().unwrap());
                alter_table_ddl.push(check_ddl);
                if let Some(comment) = comment_ddl {
                    comments_ddl.push(comment);
                }
            } else {
                let same_check = self
                    .old_checks
                    .iter()
                    .find(|c| c.id() == check.id())
                    .unwrap();

                let mut rename_check =
                    check.get_rename_ddl(same_check, self.table_name.as_ref().unwrap());
                if !rename_check.is_empty() {
                    rename_ddl.append(&mut rename_check);
                }
                let (mut alter_check, comment_ddl) = check.get_alter_ddl(
                    same_check,
                    self.schema_name.as_deref().unwrap(),
                    self.table_name.as_deref().unwrap(),
                );
                if !alter_check.is_empty() {
                    alter_table_ddl.append(&mut alter_check);
                }
                if let Some(comment) = comment_ddl {
                    comments_ddl.push(comment);
                }
            }
        });
        (rename_ddl, comments_ddl, alter_table_ddl)
    }
    fn update_commands(&self) {
        let mut cmds = if self.delete_field_dlg.is_some()
            || self.delete_index_dlg.is_some()
            || self.delete_foreign_key_dlg.is_some()
            || self.delete_trigger_dlg.is_some()
            || self.delete_check_dlg.is_some()
            || self.delete_unique_dlg.is_some()
            || self.delete_exclude_dlg.is_some()
            || self.delete_rule_dlg.is_some()
        {
            vec![
                Command {
                    name: "Cancel",
                    key: CANCEL_KEY,
                },
                Command {
                    name: "Confirm",
                    key: CONFIRM_KEY,
                },
            ]
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
        } else if let Some(dlg) = self.check_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.exclude_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.rule_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.trigger_dlg.as_ref() {
            dlg.get_commands()
        } else if let Some(dlg) = self.unique_dlg.as_ref() {
            dlg.get_commands()
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
            PanelKind::Uniques => self.get_unique_commands(),
            PanelKind::Excludes => self.get_exclude_commands(),
            PanelKind::Rules => self.get_rule_commands(),
            PanelKind::Triggers => self.get_trigger_commands(),
            PanelKind::Checks => self.get_check_commands(),
            PanelKind::Comment => self.get_comment_commands(),
            PanelKind::SQLPreview => self.get_sql_preview_commands(),
        };
        cmds.push(Command {
            name: "Save",
            key: SAVE_KEY,
        });
        cmds.push(Command {
            name: "Back",
            key: BACK_KEY,
        });
        cmds
    }
    fn get_field_commands(&self) -> Vec<Command> {
        let mut cmds = vec![];
        cmds.push(Command {
            name: "New Field",
            key: NEW_KEY,
        });
        if self.fields_state.selected().is_some() {
            cmds.append(&mut vec![
                Command {
                    name: "Edit Field",
                    key: EDIT_KEY,
                },
                Command {
                    name: "Delete Field",
                    key: DELETE_KEY,
                },
                Command {
                    name: "Move Up",
                    key: MOVE_UP_KEY,
                },
                Command {
                    name: "Move Down",
                    key: MOVE_DOWN_KEY,
                },
            ]);
        }
        cmds
    }
    fn get_index_commands(&self) -> Vec<Command> {
        let mut cmds = vec![];
        cmds.push(Command {
            name: "New Index",
            key: NEW_KEY,
        });
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
                Command {
                    name: "Move Up",
                    key: MOVE_UP_KEY,
                },
                Command {
                    name: "Move Down",
                    key: MOVE_DOWN_KEY,
                },
            ]);
        }
        cmds
    }
    fn get_foreign_key_commands(&self) -> Vec<Command> {
        let mut cmds = Vec::new();
        cmds.push(Command {
            name: "New Foreign Key",
            key: NEW_KEY,
        });
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
                Command {
                    name: "Move Up",
                    key: MOVE_UP_KEY,
                },
                Command {
                    name: "Move Down",
                    key: MOVE_DOWN_KEY,
                },
            ]);
        }
        cmds
    }
    fn get_unique_commands(&self) -> Vec<Command> {
        let mut cmds = Vec::new();
        cmds.push(Command {
            name: "New Unique",
            key: NEW_KEY,
        });
        if self.uniques_state.selected().is_some() {
            cmds.append(&mut vec![
                Command {
                    name: "Edit Unique",
                    key: EDIT_KEY,
                },
                Command {
                    name: "Delete Unique",
                    key: DELETE_KEY,
                },
                Command {
                    name: "Move Up",
                    key: MOVE_UP_KEY,
                },
                Command {
                    name: "Move Down",
                    key: MOVE_DOWN_KEY,
                },
            ]);
        }
        cmds
    }
    fn get_rule_commands(&self) -> Vec<Command> {
        let mut cmds = Vec::new();
        cmds.push(Command {
            name: "New Rule",
            key: NEW_KEY,
        });
        if self.rules_state.selected().is_some() {
            cmds.append(&mut vec![
                Command {
                    name: "Edit Rule",
                    key: EDIT_KEY,
                },
                Command {
                    name: "Delete Rule",
                    key: DELETE_KEY,
                },
                Command {
                    name: "Move Up",
                    key: MOVE_UP_KEY,
                },
                Command {
                    name: "Move Down",
                    key: MOVE_DOWN_KEY,
                },
            ]);
        }
        cmds
    }
    fn get_exclude_commands(&self) -> Vec<Command> {
        let mut cmds = Vec::new();
        cmds.push(Command {
            name: "New Exclude",
            key: NEW_KEY,
        });
        if self.excludes_state.selected().is_some() {
            cmds.append(&mut vec![
                Command {
                    name: "Edit Exclude",
                    key: EDIT_KEY,
                },
                Command {
                    name: "Delete Exclude",
                    key: DELETE_KEY,
                },
                Command {
                    name: "Move Up",
                    key: MOVE_UP_KEY,
                },
                Command {
                    name: "Move Down",
                    key: MOVE_DOWN_KEY,
                },
            ]);
        }
        cmds
    }
    fn get_trigger_commands(&self) -> Vec<Command> {
        let mut cmds = Vec::new();
        cmds.push(Command {
            name: "New Trigger",
            key: NEW_KEY,
        });
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
                Command {
                    name: "Move Up",
                    key: MOVE_UP_KEY,
                },
                Command {
                    name: "Move Down",
                    key: MOVE_DOWN_KEY,
                },
            ]);
        }
        cmds
    }
    fn get_check_commands(&self) -> Vec<Command> {
        let mut cmds = vec![];
        cmds.push(Command {
            name: "New Check",
            key: NEW_KEY,
        });
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
                Command {
                    name: "Move Up",
                    key: MOVE_UP_KEY,
                },
                Command {
                    name: "Move Down",
                    key: MOVE_DOWN_KEY,
                },
            ]);
        }
        cmds
    }
    fn get_comment_commands(&self) -> Vec<Command> {
        vec![]
    }
    fn get_sql_preview_commands(&self) -> Vec<Command> {
        vec![]
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
                        i.fields()
                            .iter()
                            .map(|f| f.to_show_string())
                            .collect::<Vec<String>>()
                            .join(","),
                        i.index_method().map(|s| s.to_string()).unwrap_or_default(),
                        i.unique().to_string(),
                        i.concurrent().to_string(),
                        i.comment.clone().unwrap_or_default(),
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec![
            "Name",
            "Fields",
            "Index Method",
            "Unique",
            "Concurrent",
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
                        f.ref_schema(),
                        f.ref_table(),
                        f.ref_field(),
                        f.on_delete().unwrap_or(""),
                        f.on_update().unwrap_or(""),
                        f.comment().unwrap_or(""),
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec![
            "Name",
            "Field",
            "Ref Schema",
            "Ref Table",
            "Ref Field",
            "On Delete",
            "On Update",
            "Comment",
        ]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
            Constraint::Ratio(1, 8),
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
                .map(|t| {
                    RowUI::new(vec![
                        t.name().to_string(),
                        t.for_each().unwrap_or("").to_string(),
                        t.fires().unwrap_or("").to_string(),
                        if t.insert() { "\u{2705}" } else { "" }.to_string(),
                        if t.update() { "\u{2705}" } else { "" }.to_string(),
                        if t.delete() { "\u{2705}" } else { "" }.to_string(),
                        if t.truncate() { "\u{2705}" } else { "" }.to_string(),
                        t.update_fields()
                            .iter()
                            .filter(|f| f.is_some())
                            .map(|f| f.as_deref().unwrap().to_string())
                            .collect::<Vec<String>>()
                            .join(","),
                        if t.enable() { "\u{2705}" } else { "" }.to_string(),
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec![
            "Name",
            "For Each",
            "Fires",
            "Insert",
            "Update",
            "Delete",
            "Truncate",
            "Update of Fields",
            "Enable",
        ]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 9),
            Constraint::Ratio(1, 9),
            Constraint::Ratio(1, 9),
            Constraint::Ratio(1, 9),
            Constraint::Ratio(1, 9),
            Constraint::Ratio(1, 9),
            Constraint::Ratio(1, 9),
            Constraint::Ratio(1, 9),
            Constraint::Ratio(1, 9),
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
                        if c.no_inherit() { "\u{2705}" } else { "" },
                        c.comment().unwrap_or(""),
                    ])
                })
                .collect::<Vec<RowUI>>(),
        )
        .header(RowUI::new(vec![
            "Name",
            "Expression",
            "No Inherit",
            "Comment",
        ]))
        .block(Block::default())
        .widths(&[
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
            Constraint::Ratio(1, 4),
        ])
        .highlight_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(table, r, &mut self.checks_state);
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
        self.schema_name = None;
        self.table_name = None;
        self.panel = PanelKind::Fields;
        self.fields = Vec::new();
        self.old_fields = Vec::new();
        self.indexes = Vec::new();
        self.old_indexes = Vec::new();
        self.foreign_keys = Vec::new();
        self.old_foreign_keys = Vec::new();
        self.uniques = Vec::new();
        self.old_uniques = Vec::new();
        self.checks = Vec::new();
        self.old_checks = Vec::new();
        self.excludes = Vec::new();
        self.old_excludes = Vec::new();
        self.rules = Vec::new();
        self.old_rules = Vec::new();
        self.triggers = Vec::new();
        self.old_triggers = Vec::new();
        self.comment = TextArea::default();
        self.old_comment = TextArea::default();
        self.sql_preview = TextArea::default();
        self.fields_state = TableState::default();
        self.indexes_state = TableState::default();
        self.foreign_keys_state = TableState::default();
        self.uniques_state = TableState::default();
        self.checks_state = TableState::default();
        self.excludes_state = TableState::default();
        self.rules_state = TableState::default();
        self.triggers_state = TableState::default();
        self.input_dlg = None;
        self.exit_dlg = None;
        self.delete_field_dlg = None;
        self.delete_index_dlg = None;
        self.delete_foreign_key_dlg = None;
        self.delete_unique_dlg = None;
        self.delete_check_dlg = None;
        self.delete_exclude_dlg = None;
        self.delete_rule_dlg = None;
        self.delete_trigger_dlg = None;
        self.field_dlg = None;
        self.index_dlg = None;
        self.foreign_key_dlg = None;
        self.unique_dlg = None;
        self.exclude_dlg = None;
        self.rule_dlg = None;
        self.check_dlg = None;
        self.trigger_dlg = None;
    }
    fn handle_exit_event(&mut self, key: &Key) -> ComponentResult {
        if let Some(dlg) = self.exit_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.exit_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    self.clear();
                    return ComponentResult::Back(MainPanel::TableListPG);
                }
                _ => (),
            }
        }
        ComponentResult::Done
    }
    fn handle_info_event(&mut self, key: &Key) -> ComponentResult {
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
    fn handle_field_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.field_dlg.as_mut() {
            match dlg.handle_event(key)? {
                DialogResult::Cancel => {
                    self.field_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    let field = Self::map_to_pg_field(&map);
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

    async fn handle_index_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.index_dlg.as_mut() {
            match dlg.handle_event(key).await? {
                DialogResult::Cancel => {
                    self.index_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    let index = Self::map_to_index(&map);
                    match dlg.get_id() {
                        None => self.indexes.push(index),
                        Some(_) => {
                            if let Some(i) = self.indexes_state.selected() {
                                self.indexes.splice(i..i + 1, [index]);
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
            match dlg.handle_event(key)? {
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
                DialogResult::Changed(name, selected) => {
                    match name.as_str() {
                        "reference schema" => {
                            let pool = get_pg_pool(
                                self.conns.clone(),
                                self.pools.clone(),
                                self.conn_id.as_ref().unwrap(),
                                self.db_name.as_deref(),
                            )
                            .await?;

                            let table_names = get_pg_table_names(&pool, selected.as_str()).await?;
                            dlg.set_ref_tables(table_names);
                        }

                        "reference table" => {
                            let ref_schema = dlg.get_ref_schema();
                            if let Some(schema) = ref_schema {
                                let pool = get_pg_pool(
                                    self.conns.clone(),
                                    self.pools.clone(),
                                    self.conn_id.as_ref().unwrap(),
                                    self.db_name.as_deref(),
                                )
                                .await?;
                                let fields =
                                    get_pg_field_names(&pool, &schema, selected.as_str()).await?;
                                dlg.set_ref_fields(fields);
                            }
                        }
                        _ => (),
                    };
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_unique_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.unique_dlg.as_mut() {
            match dlg.handle_event(key)? {
                DialogResult::Cancel => {
                    self.unique_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    let unique = Self::map_to_unique(&map);
                    match dlg.get_id() {
                        None => self.uniques.push(unique),
                        Some(_) => {
                            if let Some(index) = self.uniques_state.selected() {
                                self.uniques.splice(index..index + 1, [unique]);
                            }
                        }
                    }

                    self.unique_dlg = None;
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
    async fn handle_exclude_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.exclude_dlg.as_mut() {
            match dlg.handle_event(key).await? {
                DialogResult::Cancel => {
                    self.exclude_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    let exclude = Self::map_to_exclude(&map);
                    match dlg.get_id() {
                        None => self.excludes.push(exclude),
                        Some(_) => {
                            if let Some(index) = self.excludes_state.selected() {
                                self.excludes.splice(index..index + 1, [exclude]);
                            }
                        }
                    }

                    self.exclude_dlg = None;
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_rule_dlg_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if let Some(dlg) = self.rule_dlg.as_mut() {
            match dlg.handle_event(key)? {
                DialogResult::Cancel => {
                    self.rule_dlg = None;
                }
                DialogResult::Confirm(map) => {
                    let rule = Self::map_to_rule(&map);
                    match dlg.get_id() {
                        None => self.rules.push(rule),
                        Some(_) => {
                            if let Some(index) = self.rules_state.selected() {
                                self.rules.splice(index..index + 1, [rule]);
                            }
                        }
                    }

                    self.rule_dlg = None;
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
    fn map_to_pg_field(map: &HashMap<String, Option<String>>) -> Field {
        let not_null = map.get("not null").unwrap().as_ref().unwrap();
        let key = map.get("key").unwrap().as_ref().unwrap();

        Field {
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
            kind: FieldKind::try_from(map.get("type").unwrap().as_deref().unwrap()).unwrap(),
            not_null: not_null == "true",
            key: key == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
            default_value: map
                .get("default value")
                .map(|dv| dv.to_owned().unwrap_or_default()),
            length: {
                let length = map.get("length");
                if let Some(length) = length {
                    if let Some(length) = length {
                        length.parse::<i32>().ok()
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            decimal: {
                let decimal = map.get("decimal");
                if let Some(decimal) = decimal {
                    if let Some(deciaml) = decimal {
                        deciaml.parse::<i32>().ok()
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
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
            index_method: map
                .get("index method")
                .unwrap()
                .as_ref()
                .map(|m| IndexMethod::try_from(m.as_str()).unwrap()),
            unique: map.get("unique").unwrap().as_ref().unwrap() == "true",
            concurrent: map.get("concurrent").unwrap().as_ref().unwrap() == "true",
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
            ref_schema: map
                .get("reference schema")
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
                .map(|m| OnDeleteKind::try_from(m.as_str()).unwrap()),
            on_update: map
                .get("on update")
                .unwrap()
                .as_ref()
                .map(|u| OnUpdateKind::try_from(u.as_str()).unwrap()),
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
        }
    }
    fn map_to_unique(map: &HashMap<String, Option<String>>) -> Unique {
        Unique {
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
                .map(|s| s.to_string())
                .collect(),
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
        }
    }
    fn map_to_rule(map: &HashMap<String, Option<String>>) -> Rule {
        Rule {
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
            event: EventKind::try_from(map.get("event").unwrap().as_deref().unwrap()).unwrap(),
            do_instead: map
                .get("do instead")
                .unwrap()
                .as_ref()
                .map(|d| DoInstead::try_from(d.as_str()).unwrap()),
            where_condition: map.get("where").unwrap().as_ref().map(|s| s.to_string()),
            definition: map
                .get("definition")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
            enable: map.get("enable").unwrap().as_ref().unwrap() == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
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
            for_each: map
                .get("for each")
                .unwrap()
                .as_ref()
                .map(|s| ForEachKind::try_from(s.as_str()).unwrap()),
            fires: map
                .get("fires")
                .unwrap()
                .as_ref()
                .map(|s| FiresKind::try_from(s.as_str()).unwrap()),
            insert: map.get("insert").unwrap().as_ref().unwrap() == "true",
            update: map.get("update").unwrap().as_ref().unwrap() == "true",
            delete: map.get("delete").unwrap().as_ref().unwrap() == "true",
            truncate: map.get("truncate").unwrap().as_ref().unwrap() == "true",
            update_fields: map
                .get("update fields")
                .unwrap()
                .as_ref()
                .unwrap()
                .split(',')
                .map(|s| Some(s.to_string()))
                .collect(),
            enable: map.get("enable").unwrap().as_ref().unwrap() == "true",
            where_condition: map.get("where").unwrap().as_ref().map(|s| s.to_string()),
            fn_schema: map
                .get("function schema")
                .unwrap()
                .as_ref()
                .unwrap()
                .to_string(),
            fn_name: map
                .get("function name")
                .unwrap()
                .as_ref()
                .unwrap()
                .to_string(),
            fn_arg: map
                .get("function arg")
                .unwrap()
                .as_ref()
                .map(|s| s.to_string()),
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
            no_inherit: map.get("no inherit").unwrap().as_ref().unwrap() == "true",
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
        }
    }
    fn map_to_exclude(map: &HashMap<String, Option<String>>) -> Exclude {
        Exclude {
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
            index_method: map
                .get("index method")
                .unwrap()
                .as_ref()
                .map(|im| IndexMethod::try_from(im.as_str()).unwrap()),
            element: map
                .get("element")
                .unwrap()
                .as_ref()
                .unwrap()
                .split(',')
                .map(|s| ExcludeElement::try_from(s).unwrap())
                .collect(),
            comment: map.get("comment").unwrap().as_ref().map(|s| s.to_string()),
        }
    }
    fn handle_back_event(&mut self) {
        self.exit_dlg = Some(ConfirmDialog::new(
            ConfirmKind::Confirm,
            "Exit",
            "Be sure to exit from table definition",
        ));
    }
    async fn handle_save_event(&mut self) -> Result<()> {
        if self.table_name.is_none() {
            self.input_dlg = Some(InputDialog::new("Table Name", None));
        } else {
            let sql = self.build_sql(None);
            if !sql.is_empty() {
                execute_pg_query_unprepared(
                    self.conns.clone(),
                    self.pools.clone(),
                    &self.conn_id.unwrap(),
                    &sql,
                )
                .await?;
                self.old_fields = self.fields.clone();
                self.old_indexes = self.indexes.clone();
                self.old_foreign_keys = self.foreign_keys.clone();
                self.old_uniques = self.uniques.clone();
                self.old_checks = self.checks.clone();
                self.old_rules = self.rules.clone();
                self.old_excludes = self.excludes.clone();
                self.old_triggers = self.triggers.clone();
                self.info_dlg = Some(ConfirmDialog::new(
                    ConfirmKind::Info,
                    "Success",
                    "Save Success",
                ));
            }
        }
        Ok(())
    }
    async fn handle_main_event(&mut self, key: &Key) -> Result<ComponentResult> {
        if matches!(*key, BACK_KEY) {
            self.handle_back_event();
        } else if matches!(*key, SAVE_KEY) {
            self.handle_save_event().await?;
        } else {
            match self.panel {
                PanelKind::Fields => self.handle_panel_fields_event(key).await?,
                PanelKind::Indexes => self.handle_panel_indexes_event(key).await?,
                PanelKind::ForeignKeys => self.handle_panel_foreign_keys_event(key).await?,
                PanelKind::Uniques => self.handle_panel_unique_event(key).await?,
                PanelKind::Excludes => self.handle_panel_exclude_event(key).await?,
                PanelKind::Rules => self.handle_panel_rule_event(key).await?,
                PanelKind::Triggers => self.handle_panel_triggers_event(key).await?,
                PanelKind::Checks => self.handle_panel_checks_event(key).await?,
                PanelKind::Comment => self.handle_panel_comment_event(key).await?,
                PanelKind::SQLPreview => self.handle_panel_sql_preview_event(key).await?,
            };
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_fields_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            NEW_KEY => self.field_dlg = Some(FieldDialog::new(None)),
            CONFIRM_KEY => {
                if let Some(index) = self.fields_state.selected() {
                    self.field_dlg = Some(FieldDialog::new(Some(&self.fields[index])));
                }
            }
            DELETE_KEY => {
                if self.fields_state.selected().is_some() {
                    self.delete_field_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Warning,
                        "Delete Field",
                        "Are you sure to delete field ?",
                    ));
                }
            }
            TAB_RIGHT_KEY => {
                self.panel = PanelKind::Indexes;
            }
            TAB_LEFT_KEY => {
                self.panel = PanelKind::SQLPreview;
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
            MOVE_UP_KEY => {
                if !self.fields.is_empty() {
                    if let Some(index) = self.fields_state.selected() {
                        if index > 0 {
                            self.fields.swap(index, index - 1);
                        }
                    }
                }
            }
            MOVE_DOWN_KEY => {
                if !self.fields.is_empty() {
                    if let Some(index) = self.fields_state.selected() {
                        if index < self.fields.len() - 1 {
                            self.fields.swap(index, index + 1);
                        }
                    }
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_indexes_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            NEW_KEY => {
                let schemas = get_all_pg_schemas(
                    self.conns.clone(),
                    self.pools.clone(),
                    self.conn_id.as_ref().unwrap(),
                    self.db_name.as_deref(),
                )
                .await?;
                self.index_dlg = Some(IndexDialog::new(
                    &self.fields,
                    &schemas,
                    None,
                    self.conns.clone(),
                    self.pools.clone(),
                    self.conn_id.as_ref().unwrap(),
                ));
            }
            CONFIRM_KEY => {
                if let Some(index) = self.indexes_state.selected() {
                    let schemas = get_all_pg_schemas(
                        self.conns.clone(),
                        self.pools.clone(),
                        self.conn_id.as_ref().unwrap(),
                        self.db_name.as_deref(),
                    )
                    .await?;
                    self.index_dlg = Some(IndexDialog::new(
                        &self.fields,
                        &schemas,
                        Some(&self.indexes[index]),
                        self.conns.clone(),
                        self.pools.clone(),
                        self.conn_id.as_ref().unwrap(),
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
            TAB_RIGHT_KEY => {
                self.panel = PanelKind::ForeignKeys;
            }
            TAB_LEFT_KEY => {
                self.panel = PanelKind::Fields;
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
            MOVE_UP_KEY => {
                if !self.indexes.is_empty() {
                    if let Some(index) = self.indexes_state.selected() {
                        if index > 0 {
                            self.indexes.swap(index, index - 1);
                        }
                    }
                }
            }
            MOVE_DOWN_KEY => {
                if !self.indexes.is_empty() {
                    if let Some(index) = self.indexes_state.selected() {
                        if index < self.indexes.len() - 1 {
                            self.indexes.swap(index, index + 1);
                        }
                    }
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_unique_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            NEW_KEY => {
                self.unique_dlg = Some(UniqueDialog::new(&self.fields, None));
            }
            CONFIRM_KEY => {
                if let Some(index) = self.uniques_state.selected() {
                    self.unique_dlg =
                        Some(UniqueDialog::new(&self.fields, Some(&self.uniques[index])));
                }
            }
            DELETE_KEY => {
                if self.uniques_state.selected().is_some() {
                    self.delete_unique_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Warning,
                        "Delete Unique",
                        "Are you sure to delete unique?",
                    ));
                }
            }
            TAB_RIGHT_KEY => {
                self.panel = PanelKind::Checks;
            }
            TAB_LEFT_KEY => {
                self.panel = PanelKind::ForeignKeys;
            }
            UP_KEY => {
                if !self.uniques.is_empty() {
                    let index = get_table_up_index(self.uniques_state.selected());
                    self.uniques_state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.uniques.is_empty() {
                    let index =
                        get_table_down_index(self.uniques_state.selected(), self.uniques.len());
                    self.uniques_state.select(Some(index));
                }
            }
            MOVE_UP_KEY => {
                if !self.uniques.is_empty() {
                    if let Some(index) = self.uniques_state.selected() {
                        if index > 0 {
                            self.uniques.swap(index, index - 1);
                        }
                    }
                }
            }
            MOVE_DOWN_KEY => {
                if !self.uniques.is_empty() {
                    if let Some(index) = self.uniques_state.selected() {
                        if index < self.uniques.len() - 1 {
                            self.uniques.swap(index, index + 1);
                        }
                    }
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_exclude_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            NEW_KEY => {
                let schemas = get_all_pg_schemas(
                    self.conns.clone(),
                    self.pools.clone(),
                    self.conn_id.as_ref().unwrap(),
                    self.db_name.as_deref(),
                )
                .await?;
                self.exclude_dlg = Some(ExcludeDialog::new(
                    &self.fields,
                    &schemas,
                    None,
                    self.conns.clone(),
                    self.pools.clone(),
                    self.conn_id.as_ref().unwrap(),
                ));
            }
            CONFIRM_KEY => {
                if let Some(index) = self.excludes_state.selected() {
                    let schemas = get_pg_schemas(
                        self.conns.clone(),
                        self.pools.clone(),
                        self.conn_id.as_ref().unwrap(),
                        self.db_name.as_deref(),
                    )
                    .await?;
                    self.exclude_dlg = Some(ExcludeDialog::new(
                        &self.fields,
                        &schemas,
                        Some(&self.excludes[index]),
                        self.conns.clone(),
                        self.pools.clone(),
                        self.conn_id.as_ref().unwrap(),
                    ));
                }
            }
            DELETE_KEY => {
                if self.excludes_state.selected().is_some() {
                    self.delete_exclude_dlg = Some(ConfirmDialog::new(
                        ConfirmKind::Warning,
                        "Delete Exclude",
                        "Are you sure to delete exclude?",
                    ));
                }
            }
            TAB_RIGHT_KEY => {
                self.panel = PanelKind::Rules;
            }
            TAB_LEFT_KEY => {
                self.panel = PanelKind::Checks;
            }
            UP_KEY => {
                if !self.excludes.is_empty() {
                    let index = get_table_up_index(self.excludes_state.selected());
                    self.excludes_state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.excludes.is_empty() {
                    let index =
                        get_table_down_index(self.excludes_state.selected(), self.excludes.len());
                    self.excludes_state.select(Some(index));
                }
            }
            MOVE_UP_KEY => {
                if !self.excludes.is_empty() {
                    if let Some(index) = self.excludes_state.selected() {
                        if index > 0 {
                            self.excludes.swap(index, index - 1);
                        }
                    }
                }
            }
            MOVE_DOWN_KEY => {
                if !self.excludes.is_empty() {
                    if let Some(index) = self.excludes_state.selected() {
                        if index < self.excludes.len() - 1 {
                            self.excludes.swap(index, index + 1);
                        }
                    }
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_rule_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            NEW_KEY => {
                self.rule_dlg = Some(RuleDialog::new(None));
            }
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
                        "Are you sure to delete rule?",
                    ));
                }
            }
            TAB_RIGHT_KEY => {
                self.panel = PanelKind::Triggers;
            }
            TAB_LEFT_KEY => {
                self.panel = PanelKind::Excludes;
            }
            UP_KEY => {
                if !self.rules.is_empty() {
                    let index = get_table_up_index(self.rules_state.selected());
                    self.excludes_state.select(Some(index));
                }
            }
            DOWN_KEY => {
                if !self.rules.is_empty() {
                    let index = get_table_down_index(self.rules_state.selected(), self.rules.len());
                    self.rules_state.select(Some(index));
                }
            }
            MOVE_UP_KEY => {
                if !self.rules.is_empty() {
                    if let Some(index) = self.rules_state.selected() {
                        if index > 0 {
                            self.rules.swap(index, index - 1);
                        }
                    }
                }
            }
            MOVE_DOWN_KEY => {
                if !self.rules.is_empty() {
                    if let Some(index) = self.rules_state.selected() {
                        if index < self.rules.len() - 1 {
                            self.rules.swap(index, index + 1);
                        }
                    }
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_foreign_keys_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            NEW_KEY => {
                let ref_schemas = get_pg_schemas(
                    self.conns.clone(),
                    self.pools.clone(),
                    self.conn_id.as_ref().unwrap(),
                    self.db_name.as_deref(),
                )
                .await?;
                self.foreign_key_dlg = Some(ForeignKeyDialog::new(
                    &self.fields,
                    &ref_schemas
                        .iter()
                        .map(|s| s.name().to_string())
                        .collect::<Vec<String>>(),
                    None,
                ));
            }
            CONFIRM_KEY => {
                if let Some(index) = self.foreign_keys_state.selected() {
                    let ref_schemas = get_pg_schemas(
                        self.conns.clone(),
                        self.pools.clone(),
                        self.conn_id.as_ref().unwrap(),
                        self.db_name.as_deref(),
                    )
                    .await?;
                    self.foreign_key_dlg = Some(ForeignKeyDialog::new(
                        &self.fields,
                        &ref_schemas
                            .iter()
                            .map(|s| s.name().to_string())
                            .collect::<Vec<String>>(),
                        Some(&self.foreign_keys[index]),
                    ));
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
            TAB_RIGHT_KEY => {
                self.panel = PanelKind::Uniques;
            }
            TAB_LEFT_KEY => {
                self.panel = PanelKind::Indexes;
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
            MOVE_UP_KEY => {
                if !self.foreign_keys.is_empty() {
                    if let Some(index) = self.foreign_keys_state.selected() {
                        if index > 0 {
                            self.foreign_keys.swap(index, index - 1);
                        }
                    }
                }
            }
            MOVE_DOWN_KEY => {
                if !self.foreign_keys.is_empty() {
                    if let Some(index) = self.foreign_keys_state.selected() {
                        if index < self.foreign_keys.len() - 1 {
                            self.foreign_keys.swap(index, index + 1);
                        }
                    }
                }
            }

            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_triggers_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            NEW_KEY => {
                let schemas = get_pg_schemas(
                    self.conns.clone(),
                    self.pools.clone(),
                    self.conn_id.as_ref().unwrap(),
                    self.db_name.as_deref(),
                )
                .await?;
                self.trigger_dlg = Some(TriggerDialog::new(&self.fields, &schemas, None));
            }
            CONFIRM_KEY => {
                if let Some(idx) = self.triggers_state.selected() {
                    let schemas = get_pg_schemas(
                        self.conns.clone(),
                        self.pools.clone(),
                        self.conn_id.as_ref().unwrap(),
                        self.db_name.as_deref(),
                    )
                    .await?;

                    self.trigger_dlg = Some(TriggerDialog::new(
                        &self.fields,
                        &schemas,
                        Some(&self.triggers[idx]),
                    ));
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
            TAB_RIGHT_KEY => {
                self.panel = PanelKind::Comment;
            }
            TAB_LEFT_KEY => {
                self.panel = PanelKind::Rules;
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
            MOVE_UP_KEY => {
                if !self.triggers.is_empty() {
                    if let Some(index) = self.triggers_state.selected() {
                        if index > 0 {
                            self.triggers.swap(index, index - 1);
                        }
                    }
                }
            }
            MOVE_DOWN_KEY => {
                if !self.triggers.is_empty() {
                    if let Some(index) = self.triggers_state.selected() {
                        if index < self.triggers.len() - 1 {
                            self.triggers.swap(index, index + 1);
                        }
                    }
                }
            }

            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_checks_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
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
            TAB_RIGHT_KEY => {
                self.panel = PanelKind::Excludes;
            }
            TAB_LEFT_KEY => {
                self.panel = PanelKind::Uniques;
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
            MOVE_UP_KEY => {
                if !self.checks.is_empty() {
                    if let Some(index) = self.checks_state.selected() {
                        if index > 0 {
                            self.checks.swap(index, index - 1);
                        }
                    }
                }
            }
            MOVE_DOWN_KEY => {
                if !self.checks.is_empty() {
                    if let Some(index) = self.checks_state.selected() {
                        if index < self.checks.len() - 1 {
                            self.checks.swap(index, index + 1);
                        }
                    }
                }
            }
            _ => (),
        }
        Ok(ComponentResult::Done)
    }
    async fn handle_panel_comment_event(&mut self, key: &Key) -> Result<ComponentResult> {
        match *key {
            TAB_LEFT_KEY => self.panel = PanelKind::Triggers,
            TAB_RIGHT_KEY => self.panel = PanelKind::SQLPreview,
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
                    execute_pg_query_unprepared(
                        self.conns.clone(),
                        self.pools.clone(),
                        &self.conn_id.unwrap(),
                        &sql,
                    )
                    .await?;
                    self.clear();
                    return Ok(ComponentResult::BackRefresh(MainPanel::TableListMySQL));
                }
                _ => (),
            }
        }
        Ok(ComponentResult::Done)
    }
    fn handle_delete_field_event(&mut self, key: &Key) -> ComponentResult {
        if let Some(dlg) = self.delete_field_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_field_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.fields_state.selected() {
                        self.fields.remove(index);
                        self.fields_state.select(None);
                        self.delete_field_dlg = None;
                    }
                }
                _ => (),
            }
        }
        ComponentResult::Done
    }
    fn handle_delete_index_event(&mut self, key: &Key) -> ComponentResult {
        if let Some(dlg) = self.delete_index_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_index_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.indexes_state.selected() {
                        self.indexes.remove(index);
                        self.indexes_state.select(None);
                        self.delete_index_dlg = None;
                    }
                }
                _ => (),
            }
        }
        ComponentResult::Done
    }
    fn handle_delete_unique_event(&mut self, key: &Key) -> ComponentResult {
        if let Some(dlg) = self.delete_unique_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_unique_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.uniques_state.selected() {
                        self.uniques.remove(index);
                        self.uniques_state.select(None);
                        self.delete_unique_dlg = None;
                    }
                }
                _ => (),
            }
        }
        ComponentResult::Done
    }

    fn handle_delete_exclude_event(&mut self, key: &Key) -> ComponentResult {
        if let Some(dlg) = self.delete_exclude_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_exclude_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.excludes_state.selected() {
                        self.excludes.remove(index);
                        self.excludes_state.select(None);
                        self.delete_exclude_dlg = None;
                    }
                }
                _ => (),
            }
        }

        ComponentResult::Done
    }
    fn handle_delete_rule_event(&mut self, key: &Key) -> ComponentResult {
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
    fn handle_delete_check_event(&mut self, key: &Key) -> ComponentResult {
        if let Some(dlg) = self.delete_check_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_check_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.checks_state.selected() {
                        self.checks.remove(index);
                        self.checks_state.select(None);
                        self.delete_check_dlg = None;
                    }
                }
                _ => (),
            }
        }
        ComponentResult::Done
    }
    fn handle_delete_trigger_event(&mut self, key: &Key) -> ComponentResult {
        if let Some(dlg) = self.delete_trigger_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_trigger_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.triggers_state.selected() {
                        self.triggers.remove(index);
                        self.triggers_state.select(None);
                        self.delete_trigger_dlg = None;
                    }
                }
                _ => (),
            }
        }
        ComponentResult::Done
    }
    fn handle_delete_foreign_key_event(&mut self, key: &Key) -> ComponentResult {
        if let Some(dlg) = self.delete_foreign_key_dlg.as_mut() {
            match dlg.handle_event(key) {
                DialogResult::Cancel => {
                    self.delete_foreign_key_dlg = None;
                }
                DialogResult::Confirm(_) => {
                    if let Some(index) = self.foreign_keys_state.selected() {
                        self.foreign_keys.remove(index);
                        self.foreign_keys_state.select(None);
                        self.delete_foreign_key_dlg = None;
                    }
                }
                _ => (),
            }
        }
        ComponentResult::Done
    }
    fn schema_name(&self) -> Option<&str> {
        self.schema_name.as_deref()
    }
    fn table_name(&self) -> Option<&str> {
        self.table_name.as_deref()
    }
}
