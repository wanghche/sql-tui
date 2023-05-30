use crate::{
    app::APP_DIR,
    model::{mysql::Connection as MySQLConnection, pg::Connection as PGConnection, Connect},
};
use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, File, OpenOptions},
    io::{prelude::*, ErrorKind},
    path::PathBuf,
};
use uuid::Uuid;

const CONFIG_FILE: &str = "config";

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub version: String,
    pub mysql_connections: Vec<MySQLConnection>,
    pub pg_connections: Vec<PGConnection>,
}

impl Config {
    pub fn init() -> Result<Self> {
        let config_dir = Config::init_config_dir()?;
        let config_str = Config::read_config_file(config_dir)?;
        let config = serde_json::from_str(&config_str)?;
        Ok(config)
    }

    fn init_config_dir() -> Result<PathBuf> {
        let config_dir = dirs_next::home_dir();
        if let Some(mut dir) = config_dir {
            dir.push(APP_DIR);
            create_dir_all(dir.clone())?;
            Ok(dir)
        } else {
            Err(Error::new(std::io::Error::new(
                ErrorKind::NotFound,
                "home directory not exists.",
            )))
        }
    }
    fn read_config_file(config_dir: PathBuf) -> Result<String> {
        let mut config_file = config_dir.clone();
        config_file.push(CONFIG_FILE);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&config_file)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        if content.len() == 0 {
            let config = Config {
                version: String::from("0.1"),
                mysql_connections: Vec::new(),
                pg_connections: Vec::new(),
            };
            let json = serde_json::to_string(&config)?;
            write!(file, "{}", json.trim())?;
            Ok(json)
        } else {
            Ok(content)
        }
    }
    fn get_config_file() -> Result<File> {
        let mut path = Config::init_config_dir()?;
        path.push(CONFIG_FILE);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        Ok(file)
    }
    pub fn get_mysql_connections(&self) -> &Vec<MySQLConnection> {
        &self.mysql_connections
    }
    pub fn get_pg_connections(&self) -> &Vec<PGConnection> {
        &self.pg_connections
    }
    pub fn save_mysql_connection(&mut self, conn: &MySQLConnection) -> Result<()> {
        let index = self
            .mysql_connections
            .iter()
            .position(|c| c.get_id() == conn.get_id());
        if let Some(i) = index {
            self.mysql_connections.splice(i..i + 1, [conn.to_owned()]);
        } else {
            self.mysql_connections.push(conn.to_owned());
        }
        let mut file = Config::get_config_file()?;
        let json = serde_json::to_string(self)?;
        write!(file, "{}", json.trim())?;
        Ok(())
    }
    pub fn save_pg_connection(&mut self, conn: &PGConnection) -> Result<()> {
        let mut file = Config::get_config_file()?;
        let index = self
            .pg_connections
            .iter()
            .position(|c| c.get_id() == conn.get_id());
        if let Some(i) = index {
            self.pg_connections.splice(i..i + 1, [conn.to_owned()]);
        } else {
            self.pg_connections.push(conn.to_owned());
        }
        let json = serde_json::to_string(self)?;
        write!(file, "{}", json.trim())?;
        Ok(())
    }
    pub fn delete_mysql_connection(&mut self, conn_id: &Uuid) -> Result<()> {
        let mut file = Config::get_config_file()?;
        self.mysql_connections = self
            .mysql_connections
            .iter()
            .filter(|c| c.get_id() != conn_id)
            .cloned()
            .collect();

        let json = serde_json::to_string(self)?;
        write!(file, "")?;
        write!(file, "{}", json.trim())?;
        Ok(())
    }
    pub fn delete_pg_connection(&mut self, conn_id: &Uuid) -> Result<()> {
        let mut file = Config::get_config_file()?;
        self.pg_connections = self
            .pg_connections
            .iter()
            .filter(|c| c.get_id() != conn_id)
            .cloned()
            .collect();

        let json = serde_json::to_string(self)?;
        write!(file, "")?;
        write!(file, "{}", json.trim())?;
        Ok(())
    }
}
