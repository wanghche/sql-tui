use anyhow::Result;
use chrono::{DateTime, Local};
use sqlx::{postgres::types::Oid, PgPool, Row};

#[derive(PartialEq, Clone)]
struct U32(u32);

#[derive(Clone, PartialEq)]
pub struct Role {
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
impl Role {
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

#[derive(Clone)]
pub struct RoleMember {
    pub role_oid: Option<Oid>,
    pub role_name: Option<String>,
    pub member_oid: Option<Oid>,
    pub member_name: Option<String>,
    pub granted: bool,
    pub admin_option: bool,
}

impl RoleMember {
    pub fn get_alter_ddl(&self, old_rm: &RoleMember, is_role_left: bool) -> Option<String> {
        let left = if is_role_left {
            self.role_name.as_deref().unwrap()
        } else {
            self.member_name.as_deref().unwrap()
        };
        let right = if is_role_left {
            self.member_name.as_deref().unwrap()
        } else {
            self.role_name.as_deref().unwrap()
        };

        if self.admin_option && !old_rm.admin_option {
            Some(format!(
                r#"GRANT "{}" TO "{}" WITH ADMIN OPTION"#,
                left, right,
            ))
        } else if !self.admin_option && old_rm.admin_option {
            Some(format!(
                r#"REVOKE ADMIN OPTION FOR "{}" FROM "{}""#,
                self.role_name.as_deref().unwrap(),
                self.member_name.as_deref().unwrap()
            ))
        } else {
            if self.granted && !old_rm.granted {
                Some(format!(r#"GRANT "{}" TO "{}""#, left, right,))
            } else if !self.granted && old_rm.granted {
                Some(format!(r#"REVOKE "{}" FROM "{}""#, left, right,))
            } else {
                None
            }
        }
    }
}

pub async fn get_pg_role_names(pool: &PgPool) -> Result<Vec<String>> {
    let rs: Vec<String> = sqlx::query("SELECT rolname FROM pg_roles")
        .fetch_all(pool)
        .await?
        .iter()
        .map(|r| r.try_get("rolname").unwrap())
        .collect();

    Ok(rs)
}

pub async fn get_pg_roles(pool: &PgPool) -> Result<Vec<Role>> {
    let roles: Vec<Role> =
        sqlx::query("SELECT *,obj_description(rolname::regrole::oid) as comment FROM pg_roles")
            .fetch_all(pool)
            .await?
            .iter()
            .map(|r| Role {
                oid: r.try_get("oid").unwrap(),
                name: r.try_get("rolname").unwrap(),
                super_user: r.try_get("rolsuper").unwrap(),
                inherit: r.try_get("rolinherit").unwrap(),
                create_role: r.try_get("rolcreaterole").unwrap(),
                create_db: r.try_get("rolcreatedb").unwrap(),
                can_login: r.try_get("rolcanlogin").unwrap(),
                replication: r.try_get("rolreplication").unwrap(),
                conn_limit: r.try_get("rolconnlimit").unwrap(),
                expiry_date: r.try_get("rolvaliduntil").unwrap_or_default(),
                password: None,
                bypassrls: r.try_get("rolbypassrls").unwrap(),
                comment: r.try_get("comment").unwrap_or_default(),
            })
            .collect();

    Ok(roles)
}
pub async fn get_pg_role(pool: &PgPool, name: &str) -> Result<Role> {
    let row = sqlx::query(
        "SELECT *,obj_description(rolname::regrole::oid) as comment FROM pg_roles WHERE rolname=$1",
    )
    .bind(name)
    .fetch_one(pool)
    .await?;

    let role = Role {
        oid: row.try_get("oid").unwrap(),
        name: row.try_get("rolname").unwrap(),
        super_user: row.try_get("rolsuper").unwrap(),
        inherit: row.try_get("rolinherit").unwrap(),
        create_role: row.try_get("rolcreaterole").unwrap(),
        create_db: row.try_get("rolcreatedb").unwrap(),
        can_login: row.try_get("rolcanlogin").unwrap(),
        replication: row.try_get("rolreplication").unwrap(),
        conn_limit: row.try_get("rolconnlimit").unwrap(),
        expiry_date: row.try_get("rolvaliduntil").ok(),
        password: None,
        bypassrls: row.try_get("rolbypassrls").unwrap(),
        comment: row.try_get("comment").unwrap_or_default(),
    };

    Ok(role)
}

pub async fn get_pg_role_member_ofs(pool: &PgPool, name: &str) -> Result<Vec<RoleMember>> {
    let member_ofs: Vec<RoleMember> = sqlx::query(
        r"
        SELECT r.oid, m.admin_option, m.roleid, r1.rolname
        FROM pg_roles r
        JOIN pg_auth_members m ON m.member = r.oid
        JOIN pg_roles r1 ON r1.oid = m.roleid
        WHERE r.rolname = $1
       ",
    )
    .bind(name)
    .fetch_all(pool)
    .await?
    .iter()
    .map(|row| RoleMember {
        role_oid: row.try_get("roleid").unwrap(),
        role_name: row.try_get("rolname").unwrap(),
        member_oid: row.try_get("oid").unwrap(),
        member_name: Some(name.to_string()),
        granted: true,
        admin_option: row.try_get("admin_option").unwrap(),
    })
    .collect();

    Ok(member_ofs)
}
pub async fn get_pg_role_members(pool: &PgPool, name: &str) -> Result<Vec<RoleMember>> {
    let member_ofs: Vec<RoleMember> = sqlx::query(
        r"
        SELECT r.oid, m.admin_option, m.member, r1.rolname
        FROM pg_roles r
        JOIN pg_auth_members m ON m.roleid = r.oid
        JOIN pg_roles r1 ON r1.oid = m.member
        WHERE r.rolname = $1
       ",
    )
    .bind(name)
    .fetch_all(pool)
    .await?
    .iter()
    .map(|row| RoleMember {
        role_oid: row.try_get("oid").unwrap(),
        role_name: Some(name.to_string()),
        member_oid: row.try_get("member").unwrap(),
        member_name: row.try_get("rolname").unwrap(),
        granted: true,
        admin_option: row.try_get("admin_option").unwrap(),
    })
    .collect();

    Ok(member_ofs)
}
