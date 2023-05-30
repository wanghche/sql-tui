use anyhow::Result;
use sqlx::{PgPool, Row};
use strum::{Display, EnumIter, EnumString};

#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub name: String,
    pub owner: String,
    pub space: Option<String>,
    pub has_indexes: bool,
    pub has_rules: bool,
    pub has_triggers: bool,
    pub row_security: bool,
}

#[derive(Display, EnumIter, EnumString)]
#[strum(serialize_all = "UPPERCASE")]
pub enum RowFormat {
    Dynamic,
    Compact,
    Default,
    Compressed,
    Fixed,
    Redundant,
}

pub async fn get_pg_tables(pool: &PgPool, schema_name: &str) -> Result<Vec<Table>> {
    let tbs: Vec<Table> = sqlx::query("select * from pg_catalog.pg_tables where schemaname = $1")
        .bind(schema_name)
        .fetch_all(pool)
        .await?
        .iter()
        .map(|t| Table {
            name: t.try_get("tablename").unwrap(),
            owner: t.try_get("tableowner").unwrap(),
            space: t.try_get("tablespace").unwrap(),
            has_indexes: t.try_get("hasindexes").unwrap(),
            has_rules: t.try_get("hasrules").unwrap(),
            has_triggers: t.try_get("hastriggers").unwrap(),
            row_security: t.try_get("rowsecurity").unwrap(),
        })
        .collect();
    Ok(tbs)
}
pub async fn get_pg_table_names(pool: &PgPool, schema_name: &str) -> Result<Vec<String>> {
    let names: Vec<String> =
        sqlx::query("select tablename from pg_catalog.pg_tables where schemaname = $1")
            .bind(schema_name)
            .fetch_all(pool)
            .await?
            .iter()
            .map(|t| t.try_get("tablename").unwrap())
            .collect();
    Ok(names)
}
