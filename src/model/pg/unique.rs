use sqlx::{postgres::PgRow, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct Unique {
    pub id: Uuid,
    pub name: String,
    pub fields: Vec<String>,
    //    pub table_space: Option<String>,
    //   pub fill_factor: Option<String>,
    pub comment: Option<String>,
}

impl Unique {
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn fields(&self) -> &Vec<String> {
        &self.fields
    }
    //  pub fn table_space(&self) -> Option<&str> {
    //      self.table_space.as_deref()
    //  }
    //  pub fn fill_factor(&self) -> Option<&str> {
    //      self.fill_factor.as_deref()
    //  }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_create_ddl(&self) -> String {
        format!(
            "CONSTRAINT \"{}\" UNIQUE ({})",
            self.name(),
            self.fields()
                .iter()
                .map(|f| format!("\"{}\"", f))
                .collect::<Vec<String>>()
                .join(",")
        )
    }
    pub fn get_add_ddl(&self) -> String {
        format!("ADD {}", self.get_create_ddl())
    }
    pub fn get_drop_ddl(&self) -> String {
        format!("DROP CONSTRAINT \"{}\"", self.name())
    }
    pub fn get_alter_ddl(
        &self,
        old: &Unique,
        schema_name: &str,
        table_name: &str,
    ) -> (Vec<String>, Option<String>) {
        let comment = if old.comment() != self.comment() {
            Some(format!(
                "COMMENT ON CONSTRAINT \"{}\" ON \"{}\".\"{}\" IS '{}'",
                self.name(),
                schema_name,
                table_name,
                self.comment().unwrap_or("")
            ))
        } else {
            None
        };
        (vec![], comment)
    }
    pub fn get_rename_ddl(&self, other: &Unique, table_name: &str) -> Vec<String> {
        let mut ddl = Vec::new();
        if other.name != self.name {
            ddl.push(format!(
                "ALTER TABLE {} CONSTRAINT {} TO {}",
                table_name, other.name, self.name
            ));
        }
        ddl
    }
}
pub fn convert_show_unique_to_pg_unique(rows: Vec<PgRow>) -> Vec<Unique> {
    rows.iter()
        .map(|row| Unique {
            id: Uuid::new_v4(),
            name: row.try_get("constraint_name").unwrap(),
            fields: row.try_get("columns").unwrap(),
            comment: None,
        })
        .collect()
}
