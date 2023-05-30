use sqlx::{postgres::PgRow, Row};
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};
use uuid::Uuid;

use regex::Regex;

#[derive(EnumIter, Display, AsRefStr, EnumString, IntoStaticStr, Clone)]
#[strum(serialize_all = "UPPERCASE")]
pub enum EventKind {
    Select,
    Update,
    Insert,
    Delete,
}
#[derive(EnumIter, Display, AsRefStr, EnumString, IntoStaticStr, Clone)]
#[strum(serialize_all = "UPPERCASE")]
pub enum DoInstead {
    Also,
    Instead,
}

#[derive(Clone)]
pub struct Rule {
    pub id: Uuid,
    pub name: String,
    pub event: EventKind,
    pub do_instead: Option<DoInstead>,
    pub enable: bool,
    pub where_condition: Option<String>,
    pub definition: Option<String>,
    pub comment: Option<String>,
}

impl Rule {
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn event(&self) -> &str {
        self.event.clone().into()
    }
    pub fn do_instead(&self) -> Option<&str> {
        self.do_instead.clone().map(|s| s.into())
    }
    pub fn enable(&self) -> bool {
        self.enable
    }
    pub fn where_condition(&self) -> Option<&str> {
        self.where_condition.as_deref()
    }
    pub fn definition(&self) -> Option<&str> {
        self.definition.as_deref()
    }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_create_ddl(&self, schema_name: &str, table_name: &str) -> String {
        format!(
            "CREATE RULE {} AS ON {} TO {}.{}{} DO {}{}",
            self.name(),
            self.event(),
            schema_name,
            table_name,
            if let Some(w) = self.where_condition() {
                w
            } else {
                ""
            },
            if let Some(d) = self.do_instead() {
                d
            } else {
                ""
            },
            if let Some(d) = self.definition() {
                d
            } else {
                "NOTHING"
            }
        )
    }
    pub fn get_add_ddl(&self, schema_name: &str, table_name: &str) -> String {
        self.get_create_ddl(schema_name, table_name)
    }
    pub fn get_drop_ddl(&self, table_name: &str) -> String {
        format!("DROP RULE \"{}\" ON \"{}\"", self.name, table_name)
    }
    pub fn get_alter_ddl(
        &self,
        old: &Rule,
        schema_name: &str,
        table_name: &str,
    ) -> (Vec<String>, Option<String>) {
        let mut ddl = Vec::new();
        if old.name() != self.name() {
            ddl.push(format!(
                "ALTER RULE \"{}\" ON \"{}\".\"{}\" RENAME TO \"{}\"",
                old.name(),
                schema_name,
                table_name,
                self.name()
            ));
        }
        let comment = if old.comment() != self.comment() {
            Some(format!(
                "COMMENT ON RULE \"{}\" ON \"{}\".\"{}\" IS '{}'",
                self.name(),
                schema_name,
                table_name,
                self.comment().unwrap_or(""),
            ))
        } else {
            None
        };
        (ddl, comment)
    }
}
pub fn convert_row_to_pg_rule(rows: Vec<PgRow>) -> Vec<Rule> {
    rows.iter()
        .map(|row| {
            let def: String = row.try_get("definition").unwrap();
            let reg =
                Regex::new(r"CREATE\sRULE\s(?:\w+)\sAS\s+ON\s(?P<event>SELECT|INSERT|UPDATE|DELETE)\sTO\s(?:\w+).(?:\w+)(\s+WHERE\s\((?P<where_condition>.+)\))?\sDO\s(?P<do_instead>ALSO|INSTEAD)\s+(?P<definition>.+)")
                    .unwrap();
            let caps = reg.captures(def.as_str()).unwrap();
            let event = caps.name("event").unwrap().as_str();
            let do_instead = caps.name("do_instead");
            let where_condition = caps.name("where_condition");
            let definition = caps.name("definition");
            Rule {
                id: Uuid::new_v4(),
                name: row.try_get("rulename").unwrap(),
                event: EventKind::try_from(event).unwrap(),
                do_instead: do_instead.map(|d| DoInstead::try_from(d.as_str()).unwrap()),
                enable: false,
                where_condition: where_condition.map(|w| w.as_str().to_string()),
                definition: definition.map(|d| d.as_str().to_string()),
                comment: None,
            }
        })
        .collect()
}
