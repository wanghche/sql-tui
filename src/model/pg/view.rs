use crate::model::pg::{convert_row_to_pg_rule, Rule};
use anyhow::Result;
use sqlx::{PgPool, Row};
use strum::{Display, EnumIter};

#[derive(Clone)]
pub struct View {
    pub name: String,
    pub rules: Vec<Rule>,
    pub owner: Option<String>,
    pub definition: String,
    pub comment: String,
}

#[derive(Clone, EnumIter, Display)]
pub enum CheckOption {
    Cascaded,
    Local,
}

pub async fn get_pg_views(pool: &PgPool, schema_name: &str) -> Result<Vec<View>> {
    let views: Vec<View> = sqlx::query("SELECT * ,obj_description((schemaname||'.'||viewname)::regclass::oid) as comment FROM pg_views WHERE schemaname = $1")
        .bind(schema_name)
        .fetch_all(pool)
        .await?
        .iter()
        .map(|v| View {
            name: v.try_get("viewname").unwrap(),
            rules: Vec::new(),
            owner: v.try_get("viewowner").unwrap(),
            definition: v.try_get("definition").unwrap(),
            comment:  v.try_get("comment").unwrap_or_default(),
        })
        .collect();
    Ok(views)
}
pub async fn get_pg_view(pool: &PgPool, schema_name: &str, view_name: &str) -> Result<View> {
    let row = sqlx::query("SELECT *, obj_description((schemaname||'.'||viewname)::regclass::oid) as comment FROM pg_views WHERE schemaname = $1 AND viewname=$2")
        .bind(schema_name)
        .bind(view_name)
        .fetch_one(pool)
        .await?;
    let rule_rows = sqlx::query("SELECT * FROM pg_rules WHERE schemaname=$1 AND tablename=$2")
        .bind(schema_name)
        .bind(view_name)
        .fetch_all(pool)
        .await?;

    let rules = convert_row_to_pg_rule(rule_rows);

    Ok(View {
        name: row.try_get("viewname").unwrap(),
        owner: row.try_get("viewowner").unwrap(),
        rules,
        definition: row.try_get("definition").unwrap(),
        comment: row.try_get("comment").unwrap_or_default(),
    })
}
