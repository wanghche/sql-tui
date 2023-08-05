use crate::model::{
    mysql::{get_mysql_connection, Connections as MySQLConnections},
    pg::{get_pg_connection, Connections as PGConnections},
};
use anyhow::{Error, Result};
use sqlx::{
    mysql::MySqlPoolOptions, postgres::PgPoolOptions, Connection as Conn, Executor,
    MySqlConnection, MySqlPool, PgConnection, PgPool,
};
use sqlx::{mysql::MySqlRow, postgres::PgRow};
use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};
use uuid::Uuid;

pub type MySQLPools = HashMap<(Uuid, Option<String>), MySqlPool>;
pub type PGPools = HashMap<(Uuid, Option<String>), PgPool>;

pub fn init_pools() -> (MySQLPools, PGPools) {
    (MySQLPools::new(), PGPools::new())
}

pub async fn get_mysql_pool(
    conns: Rc<RefCell<MySQLConnections>>,
    pools: Rc<RefCell<MySQLPools>>,
    conn_id: &Uuid,
    db: Option<&str>,
) -> Result<MySqlPool> {
    let conn = get_mysql_connection(conns.clone(), conn_id)?;
    let key = (*conn_id, db.map(|d| d.to_string()));
    if !pools.borrow().contains_key(&key) {
        let uri = conn.get_pool_url(db);
        let pool = MySqlPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(2))
            .connect(&uri)
            .await?;
        pools.borrow_mut().insert(key.clone(), pool);
    }
    pools
        .borrow()
        .get(&key)
        .cloned()
        .ok_or_else(|| Error::msg("can not get mysql connection"))
}
pub async fn close_mysql_pool(
    conns: Rc<RefCell<MySQLConnections>>,
    pools: Rc<RefCell<MySQLPools>>,
    conn_id: &Uuid,
    db: Option<&str>,
) {
    if get_mysql_connection(conns, conn_id).is_ok() {
        let key = (*conn_id, db.map(|d| d.to_string()));

        if let Some(pool) = pools.borrow().get(&key) {
            pool.close().await;
        }
        pools.borrow_mut().remove(&key);
    }
}

pub async fn get_pg_pool(
    conns: Rc<RefCell<PGConnections>>,
    pools: Rc<RefCell<PGPools>>,
    conn_id: &Uuid,
    db: Option<&str>,
) -> Result<PgPool> {
    let conn = get_pg_connection(conns.clone(), conn_id)?;
    let key = (*conn_id, db.map(|d| d.to_string()));
    if !pools.borrow().contains_key(&key) {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(2))
            .connect(&conn.get_pool_url(db))
            .await?;
        pools.borrow_mut().insert(key.clone(), pool);
    }
    pools
        .borrow()
        .get(&key)
        .cloned()
        .ok_or_else(|| Error::msg("cannot get pg connection"))
}
pub async fn close_pg_pool(
    conns: Rc<RefCell<PGConnections>>,
    pools: Rc<RefCell<PGPools>>,
    conn_id: &Uuid,
    db: Option<&str>,
) {
    if get_pg_connection(conns, conn_id).is_ok() {
        let key = (*conn_id, db.map(|d| d.to_string()));

        if let Some(pool) = pools.borrow().get(&key) {
            pool.close().await;
        }
        pools.borrow_mut().remove(&key);
    }
}

pub async fn test_mysql_connection(uri: &str) -> Result<()> {
    MySqlConnection::connect(uri).await?;
    Ok(())
}

pub async fn test_pg_connection(uri: &str) -> Result<()> {
    PgConnection::connect(uri).await?;
    Ok(())
}
pub async fn execute_mysql_query(
    conns: Rc<RefCell<MySQLConnections>>,
    pools: Rc<RefCell<MySQLPools>>,
    conn_id: &Uuid,
    db: Option<&str>,
    sql: &str,
) -> Result<()> {
    let pool = get_mysql_pool(conns, pools, conn_id, db).await?;
    sqlx::query(sql).execute(&pool).await?;
    Ok(())
}

pub async fn execute_mysql_query_unprepared(
    conns: Rc<RefCell<MySQLConnections>>,
    pools: Rc<RefCell<MySQLPools>>,
    conn_id: &Uuid,
    db: Option<&str>,
    sql: &str,
) -> Result<()> {
    let pool = get_mysql_pool(conns, pools, conn_id, db).await?;
    pool.execute(sql).await?;
    Ok(())
}

pub async fn execute_pg_query(
    conns: Rc<RefCell<PGConnections>>,
    pools: Rc<RefCell<PGPools>>,
    conn_id: &Uuid,
    db: Option<&str>,
    sql: &str,
) -> Result<()> {
    let pool = get_pg_pool(conns, pools, conn_id, db).await?;
    sqlx::query(sql).execute(&pool).await?;
    Ok(())
}

pub async fn execute_pg_query_unprepared(
    conns: Rc<RefCell<PGConnections>>,
    pools: Rc<RefCell<PGPools>>,
    conn_id: &Uuid,
    sql: &str,
) -> Result<()> {
    let pool = get_pg_pool(conns, pools, conn_id, None).await?;
    pool.execute(sql).await?;
    Ok(())
}

pub async fn fetch_mysql_query(
    conns: Rc<RefCell<MySQLConnections>>,
    pools: Rc<RefCell<MySQLPools>>,
    conn_id: &Uuid,
    db_name: Option<&str>,
    sql: &str,
) -> Result<Vec<MySqlRow>> {
    let pool = get_mysql_pool(conns.clone(), pools, conn_id, db_name).await?;
    let result = sqlx::query(sql).fetch_all(&pool).await;
    match result {
        Ok(v) => Ok(v),
        Err(e) => Err(Error::from(e)),
    }
}

pub async fn fetch_one_mysql(
    conns: Rc<RefCell<MySQLConnections>>,
    pools: Rc<RefCell<MySQLPools>>,
    conn_id: &Uuid,
    db_name: Option<&str>,
    sql: &str,
) -> Result<MySqlRow> {
    let pool = get_mysql_pool(conns.clone(), pools, conn_id, db_name).await?;
    let result = sqlx::query(sql).fetch_one(&pool).await;
    match result {
        Ok(v) => Ok(v),
        Err(e) => Err(Error::from(e)),
    }
}

pub async fn fetch_one_pg(
    conns: Rc<RefCell<PGConnections>>,
    pools: Rc<RefCell<PGPools>>,
    conn_id: &Uuid,
    db_name: Option<&str>,
    sql: &str,
) -> Result<Option<PgRow>> {
    let pool = get_pg_pool(conns.clone(), pools, conn_id, db_name).await?;
    let result = sqlx::query(sql).fetch_optional(&pool).await;
    match result {
        Ok(v) => Ok(v),
        Err(e) => Err(Error::from(e)),
    }
}

pub async fn fetch_pg_query(
    conns: Rc<RefCell<PGConnections>>,
    pools: Rc<RefCell<PGPools>>,
    conn_id: &Uuid,
    db_name: Option<&str>,
    sql: &str,
) -> Result<Vec<PgRow>> {
    let pool = get_pg_pool(conns.clone(), pools.clone(), conn_id, db_name).await?;
    let result = sqlx::query(sql).fetch_all(&pool).await;
    match result {
        Ok(v) => Ok(v),
        Err(e) => Err(Error::from(e)),
    }
}
