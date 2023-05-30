mod app;
mod component;
mod config;
mod dialog;
mod event;
mod model;
mod pool;
mod widget;

use crate::app::App;
use crate::config::Config;
use crate::model::{init_connections, query::Queries};
use crate::pool::init_pools;
use anyhow::Result;
use std::{cell::RefCell, rc::Rc};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::init()?;
    let (mysql_conns, pg_conns) = init_connections(&config);
    let (mysql_pools, pg_pools) = init_pools();
    let mysql_conns = Rc::new(RefCell::new(mysql_conns));
    let pg_conns = Rc::new(RefCell::new(pg_conns));
    let mysql_pools = Rc::new(RefCell::new(mysql_pools));
    let pg_pools = Rc::new(RefCell::new(pg_pools));
    let config = Rc::new(RefCell::new(config));
    let queries = Rc::new(RefCell::new(Queries::new()?));
    let mut app = App::new(
        mysql_conns,
        pg_conns,
        mysql_pools,
        pg_pools,
        config,
        queries,
    );
    app.start().await?;
    Ok(())
}
