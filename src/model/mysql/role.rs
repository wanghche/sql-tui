use anyhow::Result;
use chrono::{DateTime, Local};
use sqlx::{postgres::types::Oid, PgPool, Row};

#[derive(PartialEq, Clone)]
struct U32(u32);

#[derive(Clone, PartialEq)]
pub struct PGRole {
    pub oid: Oid,
    pub name: String,
    pub super_user: bool,
    pub inherit: bool,
    pub create_role: bool,
    pub create_db: bool,
    pub can_login: bool,
    pub replication: bool,
    pub password: Option<String>,
    pub conn_limit: i32,
    pub bypassrls: bool,
    pub expiry_date: Option<DateTime<Local>>,
    pub comment: String,
}
impl PGRole {
    pub fn oid(&self) -> &Oid {
        &self.oid
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn super_user(&self) -> bool {
        self.super_user
    }
    pub fn inherit(&self) -> bool {
        self.inherit
    }
    pub fn create_role(&self) -> bool {
        self.create_role
    }
    pub fn create_db(&self) -> bool {
        self.create_db
    }
    pub fn can_login(&self) -> bool {
        self.can_login
    }
    pub fn replication(&self) -> bool {
        self.replication
    }
    pub fn conn_limit(&self) -> i32 {
        self.conn_limit
    }
    pub fn bypassrls(&self) -> bool {
        self.bypassrls
    }
    pub fn expiry_date(&self) -> Option<&DateTime<Local>> {
        self.expiry_date.as_ref()
    }
    pub fn comment(&self) -> &str {
        &self.comment
    }
}
