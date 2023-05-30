use crate::{
    model::pg::Connections,
    pool::{get_pg_pool, PGPools},
};
use anyhow::Result;
use sqlx::{postgres::PgRow, Row};
use std::{cell::RefCell, rc::Rc};
use uuid::Uuid;

#[derive(Clone, PartialEq, Debug)]
pub struct Schema {
    pub name: String,
    pub owner: Option<String>,
}
impl Schema {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }
    pub fn get_create_ddl(&self) -> String {
        let owner = if let Some(owner) = self.owner.as_ref() {
            format!(" AUTHORIZATION {}", owner)
        } else {
            String::new()
        };

        format!("CREATE SCHEMA {}{}", self.name, owner)
    }
    pub fn get_rename_ddl(&self, old_name: &str) -> String {
        format!("ALTER SCHEMA {} RENAME TO {}", old_name, self.name)
    }
    pub fn get_alter_owner_ddl(&self) -> String {
        let owner = if let Some(owner) = self.owner() {
            owner
        } else {
            "CURRENT_USER"
        };
        format!("ALTER SCHEMA {} OWNER TO {}", self.name, owner)
    }
}
pub async fn get_all_pg_schemas(
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<PGPools>>,
    conn_id: &Uuid,
    db_name: Option<&str>,
) -> Result<Vec<Schema>> {
    let pool = get_pg_pool(conns, pools, conn_id, db_name).await?;
    let schemas: Vec<Schema> = sqlx::query("SELECT * FROM information_schema.schemata")
        .fetch_all(&pool)
        .await?
        .iter()
        .map(|r| {
            let name = r.try_get("schema_name").unwrap();
            let owner = r.try_get("schema_owner").unwrap();
            Schema { name, owner }
        })
        .collect();

    Ok(schemas)
}

pub async fn get_pg_schemas(
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<PGPools>>,
    conn_id: &Uuid,
    db_name: Option<&str>,
) -> Result<Vec<Schema>> {
    let pool = get_pg_pool(conns, pools, conn_id, db_name).await?;
    let schemas: Vec<Schema> = sqlx::query(
        "SELECT * FROM information_schema.schemata WHERE schema_name NOT LIKE 'pg_%' AND schema_name != 'information_schema'",
    )
    .fetch_all(&pool)
    .await?
    .iter()
    .map(|r| {
        let name = r.try_get("schema_name").unwrap();
        let owner = r.try_get("schema_owner").unwrap(); 
        Schema {
            name,
            owner,
        }
    })
    .collect();

    Ok(schemas)
}
pub async fn get_pg_schema(
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<PGPools>>,
    conn_id: &Uuid,
    db_name: Option<&str>,
    name: &str,
) -> Result<Schema> {
    let pool = get_pg_pool(conns, pools, conn_id, db_name).await?;
    let schema: Schema = sqlx::query(
        "SELECT * FROM information_schema.schemata WHERE schema_name NOT LIKE 'pg_%' AND schema_name != 'information_schema' AND schema_name=$1"
    )
        .bind(name)
    .map(|r: PgRow| {
        let name = r.try_get("schema_name").unwrap();
        let owner = r.try_get("schema_owner").unwrap(); 
        Schema {
            name,
            owner,
        }
    })
    .fetch_one(&pool)
    .await?;

    Ok(schema)
}
