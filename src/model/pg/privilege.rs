use anyhow::Result;
use itertools::Itertools;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct Privilege {
    pub id: Uuid,
    pub db: String,
    pub schema: String,
    pub name: String,
    pub delete: bool,
    pub insert: bool,
    pub references: bool,
    pub select: bool,
    pub trigger: bool,
    pub truncate: bool,
    pub update: bool,
}

impl Privilege {
    pub fn get_revoke_all_ddl(&self, role_name: &str) -> String {
        format!(
            "REVOKE ALL PRIVILEGE ON \"{}\".\"{}\".\"{}\" FROM \"{}\"",
            self.db, self.schema, self.name, role_name
        )
    }
    pub fn get_grant_ddl(&self, role_name: &str) -> String {
        let mut actions = Vec::new();
        if self.delete {
            actions.push("DELETE");
        }
        if self.insert {
            actions.push("INSERT");
        }
        if self.references {
            actions.push("REFERENCES");
        }
        if self.select {
            actions.push("SELECT");
        }
        if self.trigger {
            actions.push("TRIGGER");
        }
        if self.truncate {
            actions.push("TRUNCATE");
        }
        if self.update {
            actions.push("UPDATE");
        }
        format!(
            "GRANT {} ON \"{}\".\"{}\".\"{}\" TO \"{}\"",
            actions.join(","),
            self.db,
            self.schema,
            self.name,
            role_name,
        )
    }
    pub fn get_alter_ddl(&self, old: &Privilege, role_name: &str) -> Vec<String> {
        let mut ddl = Vec::new();
        let mut grant_actions = Vec::new();
        let mut revoke_actions = Vec::new();

        if self.delete != old.delete {
            if self.delete {
                grant_actions.push("DELETE");
            } else {
                revoke_actions.push("DELETE");
            }
        }
        if self.insert != old.insert {
            if self.insert {
                grant_actions.push("INSERT");
            } else {
                revoke_actions.push("INSERT");
            }
        }
        if self.references != old.references {
            if self.references {
                grant_actions.push("REFERENCES");
            } else {
                revoke_actions.push("REFERENCES");
            }
        }
        if self.select != old.select {
            if self.select {
                grant_actions.push("SELECT");
            } else {
                revoke_actions.push("SELECT");
            }
        }
        if self.trigger != old.trigger {
            if self.trigger {
                grant_actions.push("TRIGGER");
            } else {
                revoke_actions.push("TRIGGER");
            }
        }
        if self.truncate != old.truncate {
            if self.truncate {
                grant_actions.push("TRUNCATE");
            } else {
                revoke_actions.push("TRUNCATE");
            }
        }
        if self.update != old.update {
            if self.update {
                grant_actions.push("UPDATE");
            } else {
                revoke_actions.push("UPDATE");
            }
        }

        if !grant_actions.is_empty() {
            ddl.push(format!(
                "GRANT {} ON \"{}\".\"{}\".\"{}\" TO \"{}\"",
                grant_actions.join(","),
                self.db,
                self.schema,
                self.name,
                role_name,
            ))
        }
        if !revoke_actions.is_empty() {
            ddl.push(format!(
                "REVOKE {} ON \"{}\".\"{}\".\"{}\" TO \"{}\"",
                grant_actions.join(","),
                self.db,
                self.schema,
                self.name,
                role_name,
            ))
        }
        ddl
    }
}
pub async fn get_pg_role_privileges(pool: &PgPool, role_name: &str) -> Result<Vec<Privilege>> {
    let rows = sqlx::query("SELECT * FROM information_schema.role_table_grants where grantee = $1")
        .bind(role_name)
        .fetch_all(pool)
        .await?;

    let rows_data = rows
        .iter()
        .map(|row| {
            (
                row.try_get("table_catalog").unwrap(),
                row.try_get("table_schema").unwrap(),
                row.try_get("table_name").unwrap(),
                row.try_get("privilege_type").unwrap(),
                row.try_get("is_grantable").unwrap(),
            )
        })
        .collect::<Vec<(String, String, String, String, String)>>();
    Ok(rows_data
        .iter()
        .unique_by(|(db, schema, table, _, _)| (db, schema, table))
        .map(|(db, schema, name, _, _)| Privilege {
            id: Uuid::new_v4(),
            db: db.to_string(),
            schema: schema.to_string(),
            name: name.to_string(),
            delete: rows_data.contains(&(
                db.to_owned(),
                schema.to_owned(),
                name.to_owned(),
                "DELETE".to_string(),
                "YES".to_string(),
            )),
            insert: rows_data.contains(&(
                db.to_owned(),
                schema.to_owned(),
                name.to_owned(),
                "INSERT".to_string(),
                "YES".to_string(),
            )),
            references: rows_data.contains(&(
                db.to_owned(),
                schema.to_owned(),
                name.to_owned(),
                "REFERENCES".to_string(),
                "YES".to_string(),
            )),
            select: rows_data.contains(&(
                db.to_owned(),
                schema.to_owned(),
                name.to_owned(),
                "SELECT".to_string(),
                "YES".to_string(),
            )),
            trigger: rows_data.contains(&(
                db.to_owned(),
                schema.to_owned(),
                name.to_owned(),
                "TRIGGER".to_string(),
                "YES".to_string(),
            )),
            truncate: rows_data.contains(&(
                db.to_owned(),
                schema.to_owned(),
                name.to_owned(),
                "TRUNCATE".to_string(),
                "YES".to_string(),
            )),
            update: rows_data.contains(&(
                db.to_owned(),
                schema.to_owned(),
                name.to_owned(),
                "UPDATE".to_string(),
                "YES".to_string(),
            )),
        })
        .collect::<Vec<Privilege>>())
}
