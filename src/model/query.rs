use crate::app::APP_DIR;
use anyhow::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, File, OpenOptions},
    io::{prelude::*, ErrorKind},
    path::{Path, PathBuf},
};
use uuid::Uuid;

const QUERY_DIR: &str = "query";
const CONFIG_FILE: &str = "config";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Query {
    pub id: Uuid,
    pub conn_id: Uuid,
    pub db_name: String,
    pub name: String,
    pub file_path: String,
    pub file_size: u64,
    pub created_date: Option<DateTime<Utc>>,
    pub modified_date: Option<DateTime<Utc>>,
    pub access_time: Option<DateTime<Utc>>,
}

impl Query {
    pub fn new(conn_id: &Uuid, db_name: &str, name: &str) -> Result<Self> {
        let id = Uuid::new_v4();
        let file_path = Self::get_file_path(&id)?;
        Ok(Query {
            id,
            conn_id: *conn_id,
            db_name: db_name.to_string(),
            name: name.to_string(),
            file_path,
            file_size: 0,
            created_date: None,
            modified_date: None,
            access_time: None,
        })
    }
    pub fn get_file_path(id: &Uuid) -> Result<String> {
        let mut path = dirs_next::home_dir().ok_or(Error::msg("home dir not exists"))?;
        path.push(APP_DIR);
        path.push(QUERY_DIR);
        path.push(format!("{}.sql", id.to_string()));
        Ok(path.to_str().unwrap().to_string())
    }
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn conn_id(&self) -> &Uuid {
        &self.conn_id
    }
    pub fn db_name(&self) -> &str {
        self.db_name.as_str()
    }
    pub fn save_file(&mut self, sql: &str) -> Result<&Self> {
        if Path::new(&self.file_path).exists() {
            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&self.file_path)?;
            file.write(sql.as_bytes())?;
            self.modified_date = Some(Utc::now());
            self.file_size = file.metadata().unwrap().len();
        } else {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(&self.file_path)?;
            write!(file, "{}", sql)?;
            self.created_date = Some(Utc::now());
            self.file_size = file.metadata().unwrap().len();
        };
        Ok(self)
    }
    pub fn delete_file(&mut self) -> Result<()> {
        if Path::new(&self.file_path).exists() {
            std::fs::remove_file(&self.file_path)?;
        }
        Ok(())
    }
    pub fn load_file(&mut self) -> Result<(String, &Self)> {
        if Path::new(&self.file_path).exists() {
            let mut file = OpenOptions::new().read(true).open(&self.file_path)?;
            let mut sql = String::new();
            file.read_to_string(&mut sql)?;
            self.access_time = Some(Utc::now());
            Ok((sql, self))
        } else {
            Err(Error::msg("file not exists"))
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Queries(Vec<Query>);

impl Queries {
    pub fn new() -> Result<Self> {
        let config_dir = Self::init_query_dir()?;
        let config_file = Self::read_config_file(config_dir)?;
        let queries = serde_json::from_str(&config_file)?;
        Ok(queries)
    }
    fn init_query_dir() -> Result<PathBuf> {
        let config_dir = dirs_next::home_dir();
        if let Some(mut dir) = config_dir {
            dir.push(APP_DIR);
            dir.push(QUERY_DIR);
            create_dir_all(dir.clone())?;
            Ok(dir)
        } else {
            Err(Error::new(std::io::Error::new(
                ErrorKind::NotFound,
                "home directory not exists.",
            )))
        }
    }
    fn get_config_file() -> Result<File> {
        let mut path = Self::init_query_dir()?;
        path.push(CONFIG_FILE);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        Ok(file)
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
            let queries: Vec<Query> = vec![];
            let json = serde_json::to_string(&queries)?;
            write!(file, "{}", json.trim())?;
            Ok(json)
        } else {
            Ok(content)
        }
    }

    pub fn get_query(&self, conn_id: &Uuid, db_name: &str, query_name: &str) -> Option<Query> {
        self.0
            .iter()
            .find(|q| q.conn_id() == conn_id && q.db_name() == db_name && q.name() == query_name)
            .map(|s| s.to_owned())
    }
    pub fn get_queries(&self, conn_id: &Uuid, db_name: &str) -> Vec<Query> {
        self.0
            .iter()
            .filter(|q| q.conn_id() == conn_id && q.db_name() == db_name)
            .cloned()
            .collect()
    }
    pub fn save_query(&mut self, query: &Query) -> Result<()> {
        let index = self.0.iter().position(|q| q.id() == query.id());
        if let Some(i) = index {
            self.0.splice(i..i + 1, [query.to_owned()]);
        } else {
            self.0.push(query.to_owned());
        }
        let mut file = Queries::get_config_file()?;
        let json = serde_json::to_string(self)?;
        file.write(json.as_bytes())?;
        Ok(())
    }
    pub fn delete_query(&mut self, query_id: &Uuid) -> Result<()> {
        let mut query = self.0.iter_mut().find(|q| q.id() == query_id);
        if let Some(query) = query.as_mut() {
            query.delete_file()?;
        }
        let mut file = Queries::get_config_file()?;
        self.0 = self
            .0
            .iter()
            .filter(|q| q.id() != query_id)
            .cloned()
            .collect();
        let json = serde_json::to_string(self)?;
        file.write(json.as_bytes())?;
        Ok(())
    }
}
