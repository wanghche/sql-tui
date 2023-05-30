pub mod mysql;
pub mod pg;
pub mod query;

use crate::config::Config;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use strum::{Display, EnumIter, EnumString};
use uuid::Uuid;

#[derive(Clone, PartialEq, Deserialize, Serialize, EnumString, EnumIter, Display, Default)]
pub enum DatabaseKind {
    #[default]
    MySQL,
    PostgreSQL,
}

pub trait Connect {
    fn get_id(&self) -> &Uuid;
    fn get_name(&self) -> &str;
    fn get_kind(&self) -> &DatabaseKind;
    fn get_add_at(&self) -> &DateTime<Utc>;
}

pub trait DB {
    fn name(&self) -> &str;
    fn kind(&self) -> DatabaseKind;
}

pub fn get_all_connections(
    mysql_conns: Rc<RefCell<mysql::Connections>>,
    pg_conns: Rc<RefCell<pg::Connections>>,
) -> Vec<Box<dyn Connect>> {
    let mut conns: Vec<Box<dyn Connect>> = Vec::new();
    mysql_conns
        .borrow()
        .values()
        .for_each(|conn| conns.push(Box::new(conn.to_owned())));
    pg_conns
        .borrow()
        .values()
        .for_each(|conn| conns.push(Box::new(conn.to_owned())));
    conns.sort_by_key(|c| *c.get_add_at());
    conns
}

pub fn init_connections(config: &Config) -> (mysql::Connections, pg::Connections) {
    let mysqls = config.get_mysql_connections();
    let pgs = config.get_pg_connections();

    let mut mysql_conns = HashMap::new();
    let mut pg_conns = HashMap::new();

    for conn in mysqls.iter() {
        mysql_conns.insert(*conn.get_id(), conn.to_owned());
    }
    for conn in pgs.iter() {
        pg_conns.insert(*conn.get_id(), conn.to_owned());
    }
    (mysql_conns, pg_conns)
}
