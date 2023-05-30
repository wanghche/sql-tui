use anyhow::Result;
use sqlx::{MySqlPool, Row};
use strum::{Display, EnumIter};

#[derive(Clone)]
pub struct View {
    pub name: String,
    pub check_option: Option<String>,
    pub definer: Option<String>,
    pub sql_security: Option<String>,
}

#[derive(Clone, EnumIter, Display)]
pub enum CheckOption {
    Cascaded,
    Local,
}

pub async fn get_mysql_views(pool: &MySqlPool, db: &str) -> Result<Vec<View>> {
    let views: Vec<View> = sqlx::query("SELECT * FROM VIEWS WHERE TABLE_SCHEMA = ?")
        .bind(db)
        .fetch_all(pool)
        .await?
        .iter()
        .map(|v| View {
            name: v.try_get("TABLE_NAME").unwrap(),
            check_option: v.try_get("CHECK_OPTION").unwrap(),
            definer: v.try_get("DEFINER").unwrap(),
            sql_security: v.try_get("SECURITY_TYPE").unwrap(),
        })
        .collect();
    Ok(views)
}
