use crate::{
    model::mysql::Connections,
    pool::{execute_mysql_query, MySQLPools},
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{MySqlPool, Row};
use std::{cell::RefCell, rc::Rc};
use strum::{Display, EnumIter, EnumString};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub name: String,
    pub rows: Option<u64>,
    pub engine: Option<String>,
    pub collation: Option<String>,
    pub create_date: Option<DateTime<Utc>>,
    pub modified_date: Option<DateTime<Utc>>,
    pub data_length: Option<u64>,
}

#[derive(Display, EnumIter, EnumString)]
pub enum TableEngine {
    Archive,
    BlackHole,
    Csv,
    InnoDB,
    Memory,
    MrgMYISAM,
    MyISAM,
    PerformanceSchema,
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

pub async fn get_mysql_tables(pool: &MySqlPool, db: &str) -> Result<Vec<Table>> {
    let tbs: Vec<Table> =
        sqlx::query("SELECT * FROM TABLES WHERE TABLE_TYPE = 'BASE TABLE' AND TABLE_SCHEMA = ?")
            .bind(db)
            .fetch_all(pool)
            .await?
            .iter()
            .map(|t| Table {
                name: t.try_get("TABLE_NAME").unwrap(),
                engine: t.try_get("ENGINE").unwrap_or_default(),
                rows: t.try_get("TABLE_ROWS").unwrap(),
                collation: t.try_get("TABLE_COLLATION").unwrap_or_default(),
                data_length: t.try_get("DATA_LENGTH").unwrap(),
                create_date: t.try_get("CREATE_TIME").unwrap(),
                modified_date: t.try_get("UPDATE_TIME").unwrap(),
            })
            .collect();
    Ok(tbs)
}
pub async fn get_mysql_table_names(pool: &MySqlPool, db: &str) -> Result<Vec<String>> {
    let tb_names: Vec<String> = sqlx::query(
        "SELECT TABLE_NAME FROM TABLES WHERE TABLE_TYPE = 'BASE TABLE' AND TABLE_SCHEMA = ?",
    )
    .bind(db)
    .fetch_all(pool)
    .await?
    .iter()
    .map(|t| t.try_get("TABLE_NAME").unwrap())
    .collect();
    Ok(tb_names)
}

pub async fn execute_mysql_table(
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<MySQLPools>>,
    conn_id: &Uuid,
    db_name: &str,
    sql: &str,
) -> Result<()> {
    execute_mysql_query(conns, pools, conn_id, Some(db_name), sql).await?;
    Ok(())
}
