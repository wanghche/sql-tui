use sqlx::{postgres::PgRow, Row};
use strum::{Display, EnumIter, EnumString, IntoStaticStr};
use uuid::Uuid;

#[derive(EnumString, EnumIter, Display, IntoStaticStr, Clone)]
#[strum(serialize_all = "UPPERCASE")]
pub enum TriggerTime {
    Before,
    After,
}

#[derive(EnumString, EnumIter, Display, IntoStaticStr, Clone)]
#[strum(serialize_all = "UPPERCASE")]
pub enum TriggerAction {
    Insert,
    Update,
    Delete,
}

#[derive(EnumString, EnumIter, Display, Clone, IntoStaticStr)]
pub enum ForEachKind {
    Row,
    Statement,
}

#[derive(EnumString, EnumIter, Display, Clone, IntoStaticStr)]
pub enum FiresKind {
    Before,
    After,
}

#[derive(Clone)]
pub struct Trigger {
    pub id: Uuid,
    pub name: String,
    pub for_each: Option<ForEachKind>,
    pub fires: Option<FiresKind>,
    pub insert: bool,
    pub update: bool,
    pub delete: bool,
    pub truncate: bool,
    pub update_fields: Vec<Option<String>>,
    pub enable: bool,
    pub where_condition: Option<String>,
    pub fn_schema: String,
    pub fn_name: String,
    pub fn_arg: Option<String>,
}

impl Trigger {
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn for_each(&self) -> Option<&str> {
        self.for_each.as_ref().map(|f| f.clone().into())
    }
    pub fn fires(&self) -> Option<&str> {
        self.fires.as_ref().map(|f| f.clone().into())
    }
    pub fn insert(&self) -> bool {
        self.insert
    }
    pub fn update(&self) -> bool {
        self.update
    }
    pub fn delete(&self) -> bool {
        self.delete
    }
    pub fn truncate(&self) -> bool {
        self.truncate
    }
    pub fn update_fields(&self) -> &Vec<Option<String>> {
        &self.update_fields
    }
    pub fn enable(&self) -> bool {
        self.enable
    }
    pub fn where_condition(&self) -> Option<&str> {
        self.where_condition.as_deref()
    }
    pub fn fn_schema(&self) -> &str {
        self.fn_schema.as_str()
    }
    pub fn fn_name(&self) -> &str {
        self.fn_name.as_str()
    }
    pub fn fn_arg(&self) -> Option<&str> {
        self.fn_arg.as_deref()
    }
    pub fn get_create_ddl(&self, schema_name: &str, table_name: &str) -> String {
        let mut action = Vec::new();
        if self.insert {
            action.push(String::from("INSERT"));
        }
        if self.update {
            if !self.update_fields.is_empty() {
                action.push(format!(
                    "UPDATE OF {}",
                    self.update_fields
                        .iter()
                        .filter(|f| f.is_some())
                        .map(|f| format!(r#""{}""#, f.as_deref().unwrap()))
                        .collect::<Vec<String>>()
                        .join(","),
                ));
            } else {
                action.push(String::from("UPDATE"));
            }
        }
        if self.delete {
            action.push(String::from("DELETE"));
        }
        if self.truncate {
            action.push(String::from("TRUNCATE"));
        }
        format!(
            r#"CREATE TRIGGER "{}"{}{}ON "{}"."{}" {}{} EXECUTE PROCEDURE {}.{} {};"#,
            self.name,
            self.fires().unwrap_or(" "),
            if !action.is_empty() {
                action.join("OR")
            } else {
                String::new()
            },
            schema_name,
            table_name,
            self.for_each().unwrap_or(""),
            self.where_condition().unwrap_or(""),
            self.fn_schema,
            self.fn_name,
            if let Some(arg) = self.fn_arg() {
                format!("({})", arg)
            } else {
                String::from("")
            }
        )
    }
    pub fn get_add_ddl(&self, schema_name: &str, table_name: &str) -> String {
        self.get_create_ddl(schema_name, table_name)
    }
    pub fn get_drop_ddl(&self, table_name: &str) -> String {
        format!("DROP TRIGGER {} ON {}", self.name, table_name)
    }
    pub fn get_alter_ddl(
        &self,
        old: &Trigger,
        schema_name: &str,
        table_name: &str,
    ) -> (Vec<String>, Option<String>) {
        let mut ddl = Vec::new();
        if old.name() != self.name() {
            ddl.push(format!(
                "ALTER TRIGGER \"{}\" ON \"{}\".\"{}\" RENAME TO \"{}\"",
                old.name(),
                schema_name,
                table_name,
                self.name()
            ));
        }

        (ddl, None)
    }
}
pub fn convert_row_to_pg_trigger(rows: &Vec<PgRow>) -> Vec<Trigger> {
    rows.iter()
        .map(|row| {
            let tg_type: i16 = row.try_get("tgtype").unwrap();
            let update_fields: Vec<Option<String>> = row.try_get("columns").unwrap();
            Trigger {
                id: Uuid::new_v4(),
                name: row.try_get("tgname").unwrap(),
                for_each: if tg_type & 1 > 0 {
                    Some(ForEachKind::Row)
                } else {
                    Some(ForEachKind::Statement)
                },
                fires: if tg_type & 2 > 0 {
                    Some(FiresKind::Before)
                } else {
                    Some(FiresKind::After)
                },
                insert: tg_type & 4 > 0,
                update: tg_type & 8 > 0,
                delete: tg_type & 16 > 0,
                truncate: tg_type & 32 > 0,
                update_fields,
                enable: row.try_get::<i8, _>("tgenabled").unwrap() != 'D' as i8,
                where_condition: row.try_get("tgqual").unwrap(),
                fn_schema: row.try_get("nspname").unwrap(),
                fn_name: row.try_get("proname").unwrap(),
                fn_arg: row
                    .try_get::<Option<Vec<u8>>, _>("tgargs")
                    .unwrap()
                    .map(|v| String::from_utf8(v).unwrap()),
            }
        })
        .collect()
}
