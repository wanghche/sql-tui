use anyhow::Result;
use regex::Regex;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct Privilege {
    pub id: Uuid,
    pub db: String,
    pub name: String,
    pub alter: bool,
    pub create: bool,
    pub create_view: bool,
    pub delete: bool,
    pub drop: bool,
    pub index: bool,
    pub insert: bool,
    pub references: bool,
    pub select: bool,
    pub show_view: bool,
    pub trigger: bool,
    pub update: bool,
}

impl Privilege {
    pub fn get_revoke_all_ddl(&self, user_name: &str, user_host: &str) -> String {
        let mut privs = Vec::new();
        if self.alter {
            privs.push("Alter");
        }
        if self.create {
            privs.push("Create");
        }
        if self.create_view {
            privs.push("Create View");
        }
        if self.delete {
            privs.push("Delete");
        }
        if self.drop {
            privs.push("Drop");
        }
        if self.index {
            privs.push("Index");
        }
        if self.insert {
            privs.push("Insert");
        }
        if self.references {
            privs.push("References");
        }
        if self.select {
            privs.push("Select");
        }
        if self.show_view {
            privs.push("Show View");
        }
        if self.trigger {
            privs.push("Trigger");
        }
        if self.update {
            privs.push("Update");
        }

        format!(
            "REVOKE {} ON TABLE `{}`.`{}` FROM `{}`@`{}`",
            privs.join(","),
            self.db,
            self.name,
            user_name,
            user_host,
        )
    }
    pub fn get_grant_ddl(&self, user_name: &str, user_host: &str) -> String {
        let mut actions = Vec::new();
        if self.alter {
            actions.push("Alter");
        }
        if self.create {
            actions.push("Create");
        }
        if self.create_view {
            actions.push("Create View");
        }
        if self.delete {
            actions.push("Delete");
        }
        if self.drop {
            actions.push("Drop");
        }
        if self.index {
            actions.push("Index");
        }
        if self.insert {
            actions.push("Insert");
        }
        if self.references {
            actions.push("References");
        }
        if self.select {
            actions.push("Select");
        }
        if self.show_view {
            actions.push("Show View");
        }
        if self.trigger {
            actions.push("Trigger");
        }
        if self.update {
            actions.push("Update");
        }
        format!(
            "GRANT {} ON `{}`.`{}` TO `{}`@`{}`",
            actions.join(","),
            self.db,
            self.name,
            user_name,
            user_host
        )
    }
    pub fn get_alter_ddl(&self, old: &Privilege, user_name: &str, user_host: &str) -> Vec<String> {
        let mut ddl = Vec::new();
        let mut grant_actions = Vec::new();
        let mut revoke_actions = Vec::new();

        if self.alter != old.alter {
            if self.alter {
                grant_actions.push("Alter");
            } else {
                revoke_actions.push("Alter");
            }
        }
        if self.create != old.create {
            if self.create {
                grant_actions.push("Create");
            } else {
                revoke_actions.push("Create");
            }
        }
        if self.create_view != old.create_view {
            if self.create_view {
                grant_actions.push("Create View");
            } else {
                revoke_actions.push("Create View");
            }
        }
        if self.delete != old.delete {
            if self.delete {
                grant_actions.push("Delete");
            } else {
                revoke_actions.push("Delete");
            }
        }
        if self.drop != old.drop {
            if self.drop {
                grant_actions.push("Drop");
            } else {
                revoke_actions.push("Drop");
            }
        }
        if self.index != old.index {
            if self.index {
                grant_actions.push("Index");
            } else {
                revoke_actions.push("Index");
            }
        }
        if self.insert != old.insert {
            if self.insert {
                grant_actions.push("Insert");
            } else {
                revoke_actions.push("Insert");
            }
        }
        if self.references != old.references {
            if self.references {
                grant_actions.push("References");
            } else {
                revoke_actions.push("References");
            }
        }
        if self.select != old.select {
            if self.select {
                grant_actions.push("Select");
            } else {
                revoke_actions.push("Select");
            }
        }
        if self.show_view != old.show_view {
            if self.show_view {
                grant_actions.push("Show View");
            } else {
                revoke_actions.push("Show View");
            }
        }
        if self.trigger != old.trigger {
            if self.trigger {
                grant_actions.push("Trigger");
            } else {
                revoke_actions.push("Trigger");
            }
        }
        if self.update != old.update {
            if self.update {
                grant_actions.push("Update");
            } else {
                revoke_actions.push("Update");
            }
        }

        if !grant_actions.is_empty() {
            ddl.push(format!(
                "GRANT {} ON `{}`.`{}` TO `{}`@`{}`",
                grant_actions.join(","),
                self.db,
                self.name,
                user_name,
                user_host
            ))
        }
        if !revoke_actions.is_empty() {
            ddl.push(format!(
                "REVOKE {} ON `{}`.`{}` FROM `{}`@`{}`",
                revoke_actions.join(","),
                self.db,
                self.name,
                user_name,
                user_host
            ))
        }
        ddl
    }
}
pub async fn get_mysql_user_privileges(
    pool: &MySqlPool,
    host_name: &str,
    user_name: &str,
) -> Result<Vec<Privilege>> {
    let rows = sqlx::query(&format!("SHOW GRANTS FOR '{user_name}'@'{host_name}'"))
        .fetch_all(pool)
        .await?;
    let reg = Regex::new(
        r"^GRANT\s(?P<privs>((ALTER|CREATE VIEW|CREATE|DELETE|DROP|INDEX|INSERT|REFERENCES|SELECT|SHOW VIEW|TRIGGER|UPDATE|ALL PRIVILEGES),?\s?)+)\sON\s`(?P<db>\w+)`.`(?P<name>\w+)`",
    )
    .unwrap();

    Ok(rows
        .iter()
        .filter(|row| {
            let def: String = row.try_get(0).unwrap();
            reg.captures(def.as_str()).is_some()
        })
        .map(|row| {
            let def: String = row.try_get(0).unwrap();
            let caps = reg.captures(def.as_str()).unwrap();

            let privs = caps.name("privs").unwrap().as_str();
            let db = caps.name("db").unwrap();
            let name = caps.name("name").unwrap();

            Privilege {
                id: Uuid::new_v4(),
                db: db.as_str().to_string(),
                name: name.as_str().to_string(),
                alter: privs.contains("ALTER") || privs.contains("ALL PRIVILEGES"),
                create: privs.contains("CREATE") || privs.contains("ALL PRIVILEGES"),
                create_view: privs.contains("CREATE VIEW") || privs.contains("ALL PRIVILEGES"),
                delete: privs.contains("DELETE") || privs.contains("ALL PRIVILEGES"),
                drop: privs.contains("DROP") || privs.contains("ALL PRIVILEGES"),
                index: privs.contains("INDEX") || privs.contains("ALL PRIVILEGES"),
                insert: privs.contains("INSERT") || privs.contains("ALL PRIVILEGES"),
                references: privs.contains("REFERENCES") || privs.contains("ALL PRIVILEGES"),
                select: privs.contains("SELECT") || privs.contains("ALL PRIVILEGES"),
                show_view: privs.contains("SHOW VIEW") || privs.contains("ALL PRIVILEGES"),
                trigger: privs.contains("TRIGGER") || privs.contains("ALL PRIVILEGES"),
                update: privs.contains("UPDATE") || privs.contains("ALL PRIVILEGES"),
            }
        })
        .collect::<Vec<Privilege>>())
}
