use crate::{
    model::{mysql::Connections, DatabaseKind, DB},
    pool::{get_mysql_pool, MySQLPools},
};
use anyhow::Result;
use sqlx::{mysql::MySqlRow, MySqlPool, Row};
use std::{cell::RefCell, rc::Rc};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub struct Database {
    pub name: String,
    pub character_set: Option<String>,
    pub collation: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Version {
    Eight,
    Five,
}

impl DB for Database {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> DatabaseKind {
        DatabaseKind::MySQL
    }
}

impl Database {
    pub fn character_set(&self) -> Option<&str> {
        self.character_set.as_deref()
    }
    pub fn collation(&self) -> Option<&str> {
        self.collation.as_deref()
    }
    pub fn get_create_ddl(&self) -> String {
        let charset = if let Some(charset) = self.character_set.as_ref() {
            format!(" CHARACTER SET = '{}'", charset)
        } else {
            String::new()
        };

        let collation = if let Some(collation) = self.collation.as_ref() {
            format!(" COLLATE = '{}'", collation)
        } else {
            String::new()
        };
        format!("CREATE DATABASE {}{}{}", self.name, charset, collation)
    }
    pub fn get_alter_ddl(&self, old: &Database) -> String {
        let charset = if self.character_set() != old.character_set() {
            if let Some(charset) = self.character_set.as_ref() {
                format!(" DEFAULT CHARACTER SET {}", charset)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let collation = if self.collation() != old.collation() {
            if let Some(collation) = self.collation() {
                format!(" DEFAULT COLLATE {}", collation)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        if !charset.is_empty() || !collation.is_empty() {
            format!("ALTER DATABASE {}{}{}", self.name, charset, collation)
        } else {
            String::new()
        }
    }
}

pub async fn get_mysql_database(
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<MySQLPools>>,
    conn_id: &Uuid,
    db_name: &str,
) -> Result<Database> {
    let pool = get_mysql_pool(
        conns.clone(),
        pools.clone(),
        conn_id,
        Some("information_schema"),
    )
    .await?;
    let db: Database = sqlx::query(format!("SELECT SCHEMA_NAME,DEFAULT_CHARACTER_SET_NAME,DEFAULT_COLLATION_NAME FROM information_schema.schemata WHERE SCHEMA_NAME='{}'", db_name).as_str())
        .map(|r: MySqlRow| {
            let name = r.try_get("SCHEMA_NAME").unwrap();
            let character_set = r.try_get("DEFAULT_CHARACTER_SET_NAME").unwrap();
            let collation = r.try_get("DEFAULT_COLLATION_NAME").unwrap();
            Database {
                name ,
                character_set:Some(character_set),
                collation:Some(collation) ,
            }
        })
        .fetch_one(&pool)
        .await?;

    Ok(db)
}

pub async fn get_mysql_databases(pool: &MySqlPool) -> Result<Vec<Database>> {
    let dbs: Vec<Database> = sqlx::query("SELECT SCHEMA_NAME,DEFAULT_CHARACTER_SET_NAME,DEFAULT_COLLATION_NAME FROM information_schema.schemata")
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r| {
            let name = r.try_get("SCHEMA_NAME").unwrap();
            let character_set = r.try_get("DEFAULT_CHARACTER_SET_NAME").unwrap();
            let collation = r.try_get("DEFAULT_COLLATION_NAME").unwrap();
            Database {
                name ,
                character_set:Some(character_set),
                collation:Some(collation) ,
            }
        })
        .collect();

    Ok(dbs)
}

pub async fn get_mysql_db_names(pool: &MySqlPool) -> Result<Vec<String>> {
    let dbs: Vec<String> = sqlx::query("SELECT SCHEMA_NAME FROM information_schema.schemata")
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r| r.try_get("SCHEMA_NAME").unwrap())
        .collect();

    Ok(dbs)
}
