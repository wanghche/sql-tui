use crate::{
    model::{pg::Connections, DatabaseKind, DB},
    pool::{get_pg_pool, PGPools},
};
use anyhow::Result;
use sqlx::{postgres::PgRow, PgPool, Row};
use std::{cell::RefCell, rc::Rc};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub struct Database {
    pub name: String,
    pub owner: Option<String>,
    pub collation_order: Option<String>,
    pub character_class: Option<String>,
    pub template: Option<String>,
    pub tablespace: Option<String>,
    pub connection_limit: Option<String>,
    pub allow_connection: bool,
    pub is_template: bool,
}
impl DB for Database {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> DatabaseKind {
        DatabaseKind::PostgreSQL
    }
}

impl Database {
    pub fn get_character_set(&self) -> Option<&str> {
        self.character_class.as_deref()
    }
    pub fn get_collation(&self) -> Option<&str> {
        self.collation_order.as_deref()
    }
    pub fn get_template(&self) -> Option<&str> {
        self.template.as_deref()
    }
    pub fn get_table_space(&self) -> Option<&str> {
        self.tablespace.as_deref()
    }
    pub fn get_owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }
    pub fn get_allow_conn(&self) -> bool {
        self.allow_connection
    }
    pub fn get_conn_limit(&self) -> Option<&str> {
        self.connection_limit.as_deref()
    }
    pub fn get_is_template(&self) -> bool {
        self.is_template
    }
    pub fn get_create_ddl(&self) -> String {
        let mut sql = format!("CREATE DATABASE \"{}\"", self.name);
        if let Some(owner) = self.owner.as_ref() {
            sql.push_str(&format!(" OWNER = {}", owner));
        }
        if let Some(tpl) = self.template.as_ref() {
            sql.push_str(&format!(" TEMPLATE = {}", tpl));
        }
        if let Some(collate) = self.collation_order.as_ref() {
            sql.push_str(&format!(" LC_COLLATE = '{}'", collate));
        }
        if let Some(tp) = self.character_class.as_ref() {
            sql.push_str(&format!(" LC_TYPE = '{}'", tp));
        }
        if let Some(tb_spc) = self.tablespace.as_ref() {
            sql.push_str(&format!(" TABLESPACE = {}", tb_spc));
        }
        if let Some(limit) = self.connection_limit.as_ref() {
            sql.push_str(&format!(" CONNECTION LIMIT = {}", limit));
        }

        sql.push_str(&format!(
            " ALLOW_CONNECTIONS = {} IS_TEMPLATE = {}",
            self.allow_connection, self.is_template
        ));
        sql
    }
    pub fn get_rename_ddl(&self, old_name: &str) -> String {
        format!("ALTER DATABASE {} RENAME TO {}", old_name, self.name)
    }
    pub fn get_alter_owner_ddl(&self) -> String {
        format!(
            "ALTER DATABASE {} OWNER TO {}",
            self.name,
            if let Some(owner) = self.owner.as_deref() {
                owner
            } else {
                "CURRENT_USER"
            }
        )
    }
    pub fn get_alter_tablespace_ddl(&self) -> Option<String> {
        self.tablespace
            .as_ref()
            .map(|ts| format!("ALTER DATABASE {} SET TABLESPACE {}", self.name, ts,))
    }
    pub fn get_alter_options_ddl(
        &self,
        allow_conn: bool,
        conn_limit: Option<&str>,
        is_template: bool,
    ) -> String {
        let mut sql = format!("ALTER DATABASE {}", self.name);
        if self.get_allow_conn() != allow_conn {
            sql = format!(
                "{} allow_connections {}",
                sql,
                if self.get_allow_conn() {
                    "true"
                } else {
                    "false"
                }
            );
        }
        if self.get_conn_limit() != conn_limit {
            if let Some(limit) = self.get_conn_limit() {
                sql = format!("{} connection limit {}", sql, limit);
            }
        }
        if self.get_is_template() != is_template {
            sql = format!(
                "{} is_template {}",
                sql,
                if self.get_is_template() {
                    "true"
                } else {
                    "false"
                }
            );
        }
        sql
    }
}

pub async fn get_pg_database(
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<PGPools>>,
    conn_id: &Uuid,
    db_name: &str,
) -> Result<Database> {
    let pool = get_pg_pool(conns.clone(), pools.clone(), conn_id, None).await?;
    let db: Database = sqlx::query(
        format!(
            r#"
       SELECT 
            d.datname,
            r.rolname,
            d.datctype,
            d.datcollate,
            t.spcname,
            d.datconnlimit,
            d.datallowconn,
            d.datistemplate
       FROM 
            pg_catalog.pg_database d
            join pg_catalog.pg_roles r on d.datdba = r.oid
            join pg_catalog.pg_tablespace t on d.dattablespace = t.oid
       WHERE 
            d.datname = '{}'
        "#,
            db_name
        )
        .as_str(),
    )
    .map(|r: PgRow| {
        let conn_limit: i32 = r.try_get("datconnlimit").unwrap();

        Database {
            name: r.try_get("datname").unwrap(),
            owner: r.try_get("rolname").unwrap(),
            collation_order: r.try_get("datctype").unwrap(),
            character_class: r.try_get("datcollate").unwrap(),
            template: None,
            tablespace: r.try_get("spcname").unwrap(),
            connection_limit: Some(conn_limit.to_string()),
            allow_connection: r.try_get("datallowconn").unwrap(),
            is_template: r.try_get("datistemplate").unwrap(),
        }
    })
    .fetch_one(&pool)
    .await?;

    Ok(db)
}

pub async fn get_pg_databases(pool: &PgPool) -> Result<Vec<Database>> {
    let dbs: Vec<Database> = sqlx::query(
        r#"
        SELECT
            d.datname,
            r.rolname,
            d.datctype,
            d.datcollate,
            t.spcname,
            d.datconnlimit,
            d.datallowconn,
            d.datistemplate
        FROM
            pg_catalog.pg_database d
            join pg_catalog.pg_roles r on d.datdba = r.oid
            join pg_catalog.pg_tablespace t on d.dattablespace = t.oid
        WHERE
            d.datname not like 'template%' 
        "#,
    )
    .fetch_all(pool)
    .await?
    .iter()
    .map(|r| {
        let conn_limit: i32 = r.try_get("datconnlimit").unwrap();

        Database {
            name: r.try_get("datname").unwrap(),
            owner: r.try_get("rolname").unwrap(),
            collation_order: r.try_get("datctype").unwrap(),
            character_class: r.try_get("datcollate").unwrap(),
            template: None,
            tablespace: r.try_get("spcname").unwrap(),
            connection_limit: Some(conn_limit.to_string()),
            allow_connection: r.try_get("datallowconn").unwrap(),
            is_template: r.try_get("datistemplate").unwrap(),
        }
    })
    .collect();

    Ok(dbs)
}
pub async fn get_pg_db_names(pool: &PgPool) -> Result<Vec<String>> {
    let dbs: Vec<String> = sqlx::query(
        r#"
        SELECT 
            datname
        FROM 
            pg_catalog.pg_database 
        "#,
    )
    .fetch_all(pool)
    .await?
    .iter()
    .map(|r| r.try_get("datname").unwrap())
    .collect();

    Ok(dbs)
}
