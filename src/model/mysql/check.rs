use uuid::Uuid;

#[derive(Clone)]
pub struct Check {
    pub id: Uuid,
    pub name: String,
    pub expression: String,
    pub not_enforced: bool,
}

impl Check {
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn expression(&self) -> &str {
        self.expression.as_str()
    }
    pub fn not_enforced(&self) -> bool {
        self.not_enforced
    }
    pub fn get_create_ddl(&self) -> String {
        format!(
            "CONSTRAINT `{}` CHECK ({}){}",
            self.name,
            self.expression,
            if self.not_enforced {
                " NOT ENFORCED"
            } else {
                ""
            }
        )
    }
    pub fn get_add_ddl(&self) -> String {
        format!("ADD {}", self.get_create_ddl())
    }
    pub fn get_drop_ddl(&self) -> String {
        format!("DROP CHECK `{}`", self.name)
    }
    pub fn get_change_ddl(&self, check: &Check) -> Option<String> {
        if self.not_enforced != check.not_enforced {
            Some(format!(
                "ALTER CHECK {} {}",
                self.name,
                if self.not_enforced {
                    "NOT ENFORCED"
                } else {
                    "ENFORCED"
                }
            ))
        } else {
            None
        }
    }
}
