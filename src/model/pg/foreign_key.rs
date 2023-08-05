use sqlx::{postgres::PgRow, Row};
use strum::{Display, EnumIter, EnumString, IntoStaticStr};
use uuid::Uuid;

use regex::Regex;

#[derive(EnumIter, EnumString, Display, IntoStaticStr, Clone)]
#[strum(serialize_all = "UPPERCASE")]
pub enum OnDeleteKind {
    Cascade,
    SetNull,
    NoAction,
    Restrict,
    SetDefault,
}

#[derive(EnumIter, EnumString, Display, IntoStaticStr, Clone)]
#[strum(serialize_all = "UPPERCASE")]
pub enum OnUpdateKind {
    Cascade,
    NoAction,
    Restrict,
    SetNull,
    SetDefault,
}

#[derive(Clone)]
pub struct ForeignKey {
    pub id: Uuid,
    pub name: String,
    pub field: String,
    pub ref_schema: String,
    pub ref_table: String,
    pub ref_field: String,
    pub on_delete: Option<OnDeleteKind>,
    pub on_update: Option<OnUpdateKind>,
    pub comment: Option<String>,
}

impl ForeignKey {
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn field(&self) -> &str {
        &self.field
    }
    pub fn ref_schema(&self) -> &str {
        self.ref_schema.as_str()
    }
    pub fn ref_table(&self) -> &str {
        self.ref_table.as_str()
    }
    pub fn ref_field(&self) -> &str {
        &self.ref_field
    }
    pub fn on_delete(&self) -> Option<&str> {
        self.on_delete.as_ref().map(|s| s.into())
    }
    pub fn on_update(&self) -> Option<&str> {
        self.on_update.as_ref().map(|s| s.into())
    }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_create_ddl(&self, schema_name: &str, table_name: &str) -> (String, Option<String>) {
        (
            format!(
                "CONSTRAINT \"{}\" FOREIGN KEY (\"{}\") REFERENCES \"{}\".\"{}\" (\"{}\"){}{}",
                self.name(),
                self.field(),
                self.ref_schema(),
                self.ref_table(),
                self.ref_field(),
                if let Some(d) = self.on_delete() {
                    format!(" ON DELETE {}", d)
                } else {
                    String::from("")
                },
                if let Some(u) = self.on_update() {
                    format!(" ON UPDATE {}", u)
                } else {
                    String::from("")
                },
            ),
            self.comment().map(|c| {
                format!(
                    "COMMENT ON CONSTRAINT \"{}\" ON \"{}\".\"{}\" IS '{}';",
                    self.name(),
                    schema_name,
                    table_name,
                    c
                )
            }),
        )
    }
    pub fn get_add_ddl(&self, schema_name: &str, table_name: &str) -> (String, Option<String>) {
        let (fk_ddl, comment_ddl) = self.get_create_ddl(schema_name, table_name);
        (format!("ADD {}", fk_ddl), comment_ddl)
    }
    pub fn get_drop_ddl(&self) -> String {
        format!("DROP CONSTRAINT \"{}\"", self.name)
    }
    pub fn get_alter_ddl(
        &self,
        old: &ForeignKey,
        schema_name: &str,
        table_name: &str,
    ) -> (Vec<String>, Option<String>) {
        (
            vec![],
            if old.comment != self.comment {
                Some(format!(
                    r#"COMMENT ON CONSTRAINT "{}" ON "{}"."{}" IS '{}'"#,
                    self.name(),
                    schema_name,
                    table_name,
                    self.comment().unwrap_or("")
                ))
            } else {
                None
            },
        )
    }
    pub fn get_rename_ddl(&self, other: &ForeignKey, table_name: &str) -> Vec<String> {
        let mut ddl = Vec::new();
        if other.name != self.name {
            ddl.push(format!(
                "ALTER TABLE {} RENAME CONSTRAINT {} TO {}",
                table_name, other.name, self.name,
            ));
        }
        ddl
    }
}
pub fn convert_show_fk_to_pg_fk(schema_name: &str, rows: Vec<PgRow>) -> Vec<ForeignKey> {
    rows.iter()
        .map(|row| {
            let def: String = row.try_get("def").unwrap();
            let reg = Regex::new(
                r"FOREIGN\sKEY\s\((?P<field>\w+)\)\sREFERENCES\s(?P<ref_table>\w+)\((?P<ref_field>\w+)\)(\sON\sUPDATE\s(?P<on_update>CASCADE|RESTRICT|SET NULL|SET DEFAULT))?(\sON\sDELETE\s(?P<on_delete>CASCADE|RESTRICT|SET NULL|SET DEFAULT))?",
            )
            .unwrap();
            let caps = reg.captures(def.as_str()).unwrap();
            let field = caps.name("field").unwrap().as_str();
            let ref_schema = schema_name.to_string();
            let ref_table =caps.name("ref_table").unwrap().as_str(); 
            let ref_field =caps.name("ref_field").unwrap().as_str(); 
            let on_update = caps.name("on_update").map(|s| 
                s.as_str().chars().filter(|c| !c.is_whitespace()).collect::<String>());
            let on_delete = caps.name("on_delete").map(|s|s.as_str().chars().filter(|c| !c.is_whitespace()).collect::<String>());

            ForeignKey {
                id: Uuid::new_v4(),
                name: row.try_get("foreign_key").unwrap(),
                field: field.to_string(),
                ref_schema: ref_schema.to_string(),
                ref_table: ref_table.to_string(),
                ref_field: ref_field.to_string(),
                on_delete: on_delete.map(|d| OnDeleteKind::try_from(d.as_str()).unwrap()),
                on_update: on_update.map(|u| OnUpdateKind::try_from(u.as_str()).unwrap()),
                comment: row.try_get("comment").unwrap(),
            }
        })
        .collect()
}
