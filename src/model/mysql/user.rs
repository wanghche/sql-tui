use anyhow::Result;
use sqlx::{mysql::MySqlRow, MySqlPool, Row};

#[derive(Clone, Debug)]
pub struct User {
    pub host: String,
    pub name: String,
    pub plugin: Option<String>,
    pub password: Option<String>,
    pub max_queries: Option<u32>,
    pub max_updates: Option<u32>,
    pub max_connections: Option<u32>,
    pub max_user_connections: Option<u32>,
    pub alter: bool,
    pub alter_routine: bool,
    pub create: bool,
    pub create_routine: bool,
    pub create_temp_tables: bool,
    pub create_user: bool,
    pub create_view: bool,
    pub delete: bool,
    pub drop: bool,
    pub event: bool,
    pub execute: bool,
    pub file: bool,
    pub grant_option: bool,
    pub index: bool,
    pub insert: bool,
    pub lock_tables: bool,
    pub process: bool,
    pub references: bool,
    pub reload: bool,
    pub replication_client: bool,
    pub replication_slave: bool,
    pub select: bool,
    pub show_databases: bool,
    pub show_view: bool,
    pub shutdown: bool,
    pub super_priv: bool,
    pub trigger: bool,
    pub update: bool,
}

impl User {
    pub fn host(&self) -> &str {
        &self.host
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn plugin(&self) -> Option<&str> {
        self.plugin.as_deref()
    }
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }
    pub fn max_queries(&self) -> Option<String> {
        self.max_queries.map(|m| m.to_string())
    }
    pub fn max_updates(&self) -> Option<String> {
        self.max_updates.map(|m| m.to_string())
    }
    pub fn max_connections(&self) -> Option<String> {
        self.max_connections.map(|m| m.to_string())
    }
    pub fn max_user_connections(&self) -> Option<String> {
        self.max_user_connections.map(|m| m.to_string())
    }
}

#[derive(Clone)]
pub struct UserMember {
    pub user_name: Option<String>,
    pub user_host: Option<String>,
    pub member_name: Option<String>,
    pub member_host: Option<String>,
    pub granted: bool,
}

impl UserMember {
    pub fn user_name(&self) -> Option<&str> {
        self.user_name.as_deref()
    }
    pub fn user_host(&self) -> Option<&str> {
        self.user_host.as_deref()
    }
    pub fn member_name(&self) -> Option<&str> {
        self.member_name.as_deref()
    }
    pub fn member_host(&self) -> Option<&str> {
        self.member_host.as_deref()
    }
    pub fn get_alter_ddl(&self, old_rm: &UserMember, name: &str, host: &str) -> Option<String> {
        if self.granted != old_rm.granted {
            if self.granted {
                if let (Some(user_name), Some(user_host)) = (self.user_name(), self.user_host()) {
                    Some(format!(
                        "GRANT `{}`@`{}` TO `{}`@`{}`",
                        name, host, user_name, user_host
                    ))
                } else if let (Some(user_name), Some(user_host)) =
                    (self.member_name(), self.member_host())
                {
                    Some(format!(
                        "GRANT `{}`@`{}` TO `{}`@`{}`",
                        user_name, user_host, name, host
                    ))
                } else {
                    None
                }
            } else if let (Some(user_name), Some(user_host)) = (self.user_name(), self.user_host())
            {
                Some(format!(
                    "REVOKE `{}`@`{}` FROM `{}`@`{}`",
                    name, host, user_name, user_host
                ))
            } else if let (Some(user_name), Some(user_host)) =
                (self.member_name(), self.member_host())
            {
                Some(format!(
                    "REVOKE `{}`@`{}` FROM `{}`@`{}`",
                    user_name, user_host, name, host
                ))
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub async fn get_mysql_users(pool: &MySqlPool) -> Result<Vec<User>> {
    let roles: Vec<User> = sqlx::query("SELECT * FROM mysql.user")
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r| row_to_user(r))
        .collect();

    Ok(roles)
}

pub async fn get_mysql_user(pool: &MySqlPool, host: &str, user: &str) -> Result<User> {
    let row = sqlx::query("SELECT * FROM mysql.user WHERE User = ? AND Host = ?")
        .bind(user)
        .bind(host)
        .fetch_one(pool)
        .await?;
    Ok(row_to_user(&row))
}

pub async fn get_mysql_user_member_ofs(
    pool: &MySqlPool,
    host: &str,
    user: &str,
) -> Result<Vec<UserMember>> {
    let member_ofs: Vec<UserMember> = sqlx::query(
        r"
        SELECT * 
        FROM mysql.role_edges
        WHERE TO_HOST = ? AND TO_USER = ? 
       ",
    )
    .bind(host)
    .bind(user)
    .fetch_all(pool)
    .await?
    .iter()
    .map(|row| UserMember {
        user_name: row.try_get("FROM_USER").unwrap(),
        user_host: row.try_get("FROM_HOST").unwrap(),
        member_name: row.try_get("TO_USER").unwrap(),
        member_host: row.try_get("TO_HOST").unwrap(),
        granted: true,
    })
    .collect();

    Ok(member_ofs)
}
pub async fn get_mysql_user_members(
    pool: &MySqlPool,
    host: &str,
    user: &str,
) -> Result<Vec<UserMember>> {
    let member_ofs: Vec<UserMember> = sqlx::query(
        r"
        SELECT * 
        FROM mysql.role_edges 
        WHERE FROM_HOST = ? AND FROM_USER = ?
       ",
    )
    .bind(host)
    .bind(user)
    .fetch_all(pool)
    .await?
    .iter()
    .map(|row| UserMember {
        user_name: row.try_get("FROM_USER").unwrap(),
        user_host: row.try_get("FROM_HOST").unwrap(),
        member_name: row.try_get("TO_USER").unwrap(),
        member_host: row.try_get("TO_HOST").unwrap(),
        granted: true,
    })
    .collect();

    Ok(member_ofs)
}
fn row_to_user(r: &MySqlRow) -> User {
    User {
        name: r.try_get("User").unwrap(),
        host: r.try_get("Host").unwrap(),
        plugin: r.try_get("plugin").unwrap(),
        password: None,
        max_queries: r.try_get("max_questions").unwrap(),
        max_updates: r.try_get("max_updates").unwrap(),
        max_connections: r.try_get("max_connections").unwrap(),
        max_user_connections: r.try_get("max_user_connections").unwrap(),
        alter: r.try_get::<String, _>("Alter_priv").unwrap() == "Y",
        alter_routine: r.try_get::<String, _>("Alter_routine_priv").unwrap() == "Y",
        create: r.try_get::<String, _>("Create_priv").unwrap() == "Y",
        create_routine: r.try_get::<String, _>("Create_routine_priv").unwrap() == "Y",
        create_temp_tables: r.try_get::<String, _>("Create_tmp_table_priv").unwrap() == "Y",
        create_user: r.try_get::<String, _>("Create_user_priv").unwrap() == "Y",
        create_view: r.try_get::<String, _>("Create_view_priv").unwrap() == "Y",
        delete: r.try_get::<String, _>("Delete_priv").unwrap() == "Y",
        drop: r.try_get::<String, _>("Drop_priv").unwrap() == "Y",
        event: r.try_get::<String, _>("Event_priv").unwrap() == "Y",
        execute: r.try_get::<String, _>("Execute_priv").unwrap() == "Y",
        file: r.try_get::<String, _>("File_priv").unwrap() == "Y",
        grant_option: r.try_get::<String, _>("Grant_priv").unwrap() == "Y",
        index: r.try_get::<String, _>("Index_priv").unwrap() == "Y",
        insert: r.try_get::<String, _>("Insert_priv").unwrap() == "Y",
        lock_tables: r.try_get::<String, _>("Lock_tables_priv").unwrap() == "Y",
        process: r.try_get::<String, _>("Process_priv").unwrap() == "Y",
        references: r.try_get::<String, _>("References_priv").unwrap() == "Y",
        reload: r.try_get::<String, _>("Reload_priv").unwrap() == "Y",
        replication_client: r.try_get::<String, _>("Repl_client_priv").unwrap() == "Y",
        replication_slave: r.try_get::<String, _>("Repl_slave_priv").unwrap() == "Y",
        select: r.try_get::<String, _>("Select_priv").unwrap() == "Y",
        show_databases: r.try_get::<String, _>("Show_db_priv").unwrap() == "Y",
        show_view: r.try_get::<String, _>("Show_view_priv").unwrap() == "Y",
        shutdown: r.try_get::<String, _>("Shutdown_priv").unwrap() == "Y",
        super_priv: r.try_get::<String, _>("Super_priv").unwrap() == "Y",
        trigger: r.try_get::<String, _>("Trigger_priv").unwrap() == "Y",
        update: r.try_get::<String, _>("Update_priv").unwrap() == "Y",
    }
}
