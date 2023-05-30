use strum::{Display, EnumIter, EnumString, IntoStaticStr};
use uuid::Uuid;

#[derive(EnumString, EnumIter, Display, IntoStaticStr, Clone, PartialEq)]
#[strum(serialize_all = "UPPERCASE")]
pub enum TriggerTime {
    Before,
    After,
}

#[derive(EnumString, EnumIter, Display, IntoStaticStr, Clone, PartialEq)]
#[strum(serialize_all = "UPPERCASE")]
pub enum TriggerAction {
    Insert,
    Update,
    Delete,
}

#[derive(Clone)]
pub struct Trigger {
    pub id: Uuid,
    pub name: String,
    pub time: TriggerTime,
    pub action: TriggerAction,
    pub statement: String,
}

impl Trigger {
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn time(&self) -> &str {
        self.time.clone().into()
    }
    pub fn action(&self) -> &str {
        self.action.clone().into()
    }
    pub fn statement(&self) -> &str {
        self.statement.as_str()
    }
    pub fn get_create_ddl(&self, table_name: &str) -> String {
        format!(
            "CREATE TRIGGER `{}` {} {} ON `{}` FOR EACH ROW {};",
            self.name, self.time, self.action, table_name, self.statement
        )
    }
    pub fn get_drop_ddl(&self) -> String {
        format!("DROP TRIGGER `{}`;", self.name)
    }
    pub fn get_alter_ddl(&self, old_trigger: &Trigger, table_name: &str) -> Vec<String> {
        let mut ddl = Vec::new();
        if self.name != old_trigger.name
            || self.time != old_trigger.time
            || self.action != old_trigger.action
            || self.statement != old_trigger.statement
        {
            ddl.push(old_trigger.get_drop_ddl());
            ddl.push(self.get_create_ddl(table_name));
        }
        ddl
    }
}

#[derive(EnumString, EnumIter, Display, Clone, IntoStaticStr)]
pub enum ForEachKind {
    Row,
    Statement,
}

#[derive(EnumString, EnumIter, Display, Clone, IntoStaticStr)]
pub enum FiresKind {
    Before,
    After,
}
