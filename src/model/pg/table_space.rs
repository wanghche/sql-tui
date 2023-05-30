use anyhow::Result;
use sqlx::PgPool;
use sqlx::Row;

pub async fn get_pg_table_spaces(pool: &PgPool) -> Result<Vec<String>> {
    let ts: Vec<String> = sqlx::query("SELECT spcname FROM pg_tablespace")
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r| r.try_get("spcname").unwrap())
        .collect();

    Ok(ts)
}
