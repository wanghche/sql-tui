use sqlx::{postgres::PgRow, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct Unique {
    pub id: Uuid,
    pub name: String,
    pub fields: Vec<String>,
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
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_create_ddl(&self, schema_name: &str, table_name: &str) -> (String, Option<String>) {
        (
            format!(
                "CONSTRAINT \"{}\" UNIQUE ({})",
                self.name(),
                self.fields()
                    .iter()
                    .map(|f| format!("\"{}\"", f))
                    .collect::<Vec<String>>()
                    .join(",")
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
        let (un_ddl, comment_ddl) = self.get_create_ddl(schema_name, table_name);
        (format!("ADD {}", un_ddl), comment_ddl)
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
            comment: row.try_get("comment").unwrap(),
        })
        .collect()
}
