use sqlx::{mysql::MySqlRow, Row};
use strum::{Display, EnumIter, EnumString, IntoStaticStr};
use uuid::Uuid;

#[derive(EnumIter, EnumString, Display, IntoStaticStr, Clone, PartialEq)]
#[strum(serialize_all = "UPPERCASE")]
pub enum OnDeleteKind {
    Cascade,
    #[strum(serialize = "SET NULL")]
    SetNull,
    #[strum(serialize = "NO ACTION")]
    NoAction,
    Restrict,
    #[strum(serialize = "SET DEFAULT")]
    SetDefault,
}

#[derive(EnumIter, EnumString, Display, IntoStaticStr, Clone, PartialEq)]
#[strum(serialize_all = "UPPERCASE")]
pub enum OnUpdateKind {
    Cascade,
    #[strum(serialize = "NO ACTION")]
    NoAction,
    Restrict,
    #[strum(serialize = "SET NULL")]
    SetNull,
    #[strum(serialize = "SET DEFAULT")]
    SetDefault,
}

#[derive(Clone)]
pub struct ForeignKey {
    pub id: Uuid,
    pub name: String,
    pub field: String,
    pub ref_db: String,
    pub ref_table: String,
    pub ref_field: String,
    pub on_delete: Option<OnDeleteKind>,
    pub on_update: Option<OnUpdateKind>,
}
impl ForeignKey {
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn field(&self) -> &str {
        self.field.as_str()
    }
    pub fn ref_db(&self) -> &str {
        self.ref_db.as_str()
    }
    pub fn ref_table(&self) -> &str {
        self.ref_table.as_str()
    }
    pub fn ref_field(&self) -> &str {
        self.ref_field.as_str()
    }
    pub fn on_delete(&self) -> Option<&str> {
        self.on_delete.clone().map(|s| s.into())
    }
    pub fn on_update(&self) -> Option<&str> {
        self.on_update.clone().map(|s| s.into())
    }
    pub fn get_create_ddl(&self) -> String {
        let mut sql = format!(
            "CONSTRAINT `{}` FOREIGN KEY (`{}`) REFERENCES `{}`.`{}` (`{}`)",
            self.name, self.field, self.ref_db, self.ref_table, self.ref_field,
        );
        if let Some(update) = self.on_update.as_ref() {
            sql = format!("{} ON DELETE {}", sql, update);
        }
        if let Some(delete) = self.on_delete.as_ref() {
            sql = format!("{} ON UPDATE {}", sql, delete);
        }
        sql
    }

    pub fn get_drop_ddl(&self) -> String {
        format!("DROP FOREIGN KEY `{}`", self.name)
    }
    pub fn get_alter_ddl(&self, old_fk: &ForeignKey) -> Vec<String> {
        let mut ddl = Vec::new();
        if self.name != old_fk.name
            || self.field != old_fk.field
            || self.ref_db != old_fk.ref_db
            || self.ref_table != old_fk.ref_table
            || self.ref_field != old_fk.ref_field
            || self.on_delete != old_fk.on_delete
            || self.on_update != old_fk.on_update
        {
            ddl.push(old_fk.get_drop_ddl());
            ddl.push(self.get_add_ddl());
        }
        ddl
    }
    pub fn get_add_ddl(&self) -> String {
        format!("ADD {}", self.get_create_ddl())
    }
}
pub fn convert_show_fk_to_mysql_fk(rows: Vec<MySqlRow>) -> Vec<ForeignKey> {
    rows.iter()
        .map(|row| ForeignKey {
            id: Uuid::new_v4(),
            name: row.try_get("CONSTRAINT_NAME").unwrap(),
            field: row.try_get("COLUMN_NAME").unwrap(),
            ref_db: row.try_get("REFERENCED_TABLE_SCHEMA").unwrap(),
            ref_table: row.try_get("REFERENCED_TABLE_NAME").unwrap(),
            ref_field: row.try_get("REFERENCED_COLUMN_NAME").unwrap(),
            on_update: None,
            on_delete: None,
        })
        .collect()
}
