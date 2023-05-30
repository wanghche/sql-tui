use crate::{
    config::Config,
    model::{Connect, DatabaseKind},
    pool::test_mysql_connection,
};
use anyhow::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use uuid::Uuid;

pub type Connections = HashMap<Uuid, Connection>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Connection {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub add_at: DateTime<Utc>,
}

impl Connect for Connection {
    fn get_id(&self) -> &Uuid {
        &self.id
    }
    fn get_name(&self) -> &str {
        &self.name
    }
    fn get_kind(&self) -> &DatabaseKind {
        &DatabaseKind::MySQL
    }
    fn get_add_at(&self) -> &DateTime<Utc> {
        &self.add_at
    }
}

impl Connection {
    pub fn get_host(&self) -> &str {
        &self.host
    }
    pub fn get_port(&self) -> &str {
        &self.port
    }
    pub fn get_user(&self) -> &str {
        &self.user
    }
    pub fn get_password(&self) -> &str {
        &self.password
    }
    pub fn get_pool_url(&self, db_name: Option<&str>) -> String {
        if let Some(db_name) = db_name {
            format!(
                "mysql://{}:{}@{}:{}/{}",
                self.user, self.password, self.host, self.port, db_name
            )
        } else {
            format!(
                "mysql://{}:{}@{}:{}",
                self.user, self.password, self.host, self.port
            )
        }
    }
}

pub fn get_mysql_connection(
    connections: Rc<RefCell<Connections>>,
    conn_id: &Uuid,
) -> Result<Connection> {
    connections
        .borrow()
        .get(conn_id)
        .map(|c| c.to_owned())
        .ok_or(Error::msg("cannot get mysql connection"))
}

pub async fn save_mysql_connection<'a>(
    connections: Rc<RefCell<Connections>>,
    config: Rc<RefCell<Config>>,
    conn: &Connection,
) -> Result<()> {
    test_mysql_connection(&conn.get_pool_url(None)).await?;
    connections
        .borrow_mut()
        .insert(*conn.get_id(), conn.to_owned());
    config.borrow_mut().save_mysql_connection(conn)
}

pub fn delete_mysql_connection(
    connections: Rc<RefCell<Connections>>,
    conn_id: &Uuid,
    config: Rc<RefCell<Config>>,
) -> Result<()> {
    connections.borrow_mut().remove(conn_id);
    config.borrow_mut().delete_mysql_connection(conn_id)
}
