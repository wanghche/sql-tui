use sqlx::{postgres::PgRow, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct Check {
    pub id: Uuid,
    pub name: String,
    pub expression: String,
    pub no_inherit: bool,
    pub comment: Option<String>,
}

impl Check {
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn expression(&self) -> &str {
        self.expression.as_str()
    }
    pub fn no_inherit(&self) -> bool {
        self.no_inherit
    }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_create_ddl(&self) -> String {
        format!(
            "CONSTRAINT \"{}\" CHECK ({}){}",
            self.name,
            self.expression,
            if self.no_inherit() { " NO INHERIT" } else { "" }
        )
    }
    pub fn get_add_ddl(&self) -> String {
        format!("ADD {}", self.get_create_ddl())
    }
    pub fn get_drop_ddl(&self) -> String {
        format!("DROP CONSTRAINT \"{}\"", self.name)
    }
    pub fn get_alter_ddl(
        &self,
        old: &Check,
        schema_name: &str,
        table_name: &str,
    ) -> (Vec<String>, Option<String>) {
        let comment = if old.comment() != self.comment() {
            Some(format!(
                "COMMENT ON \"{}\" ON \"{}\".\"{}\" IS '{}'",
                self.name,
                schema_name,
                table_name,
                self.comment().unwrap_or("")
            ))
        } else {
            None
        };
        (vec![], comment)
    }
    pub fn get_rename_ddl(&self, other: &Check, table_name: &str) -> Vec<String> {
        let mut ddl = Vec::new();
        if other.name != self.name {
            ddl.push(format!(
                "ALTER TABLE {} RENAME CONSTRAINT {} TO {}",
                table_name, other.name, self.name
            ));
        }
        ddl
    }
}
pub fn convert_row_to_pg_check(rows: Vec<PgRow>) -> Vec<Check> {
    rows.iter()
        .map(|row| {
            let expression: String = row.try_get("def").unwrap();
            let no_inherit = expression.contains("NO INHERIT");
            Check {
                id: Uuid::new_v4(),
                name: row.try_get("constraint_name").unwrap(),
                expression,
                no_inherit,
                comment: None,
            }
        })
        .collect()
}
