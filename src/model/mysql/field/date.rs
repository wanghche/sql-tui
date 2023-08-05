use super::{comment, default_value, not_null};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct DateField {
    pub id: Uuid,
    pub name: String,
    pub not_null: bool,
    pub key: bool,
    pub comment: Option<String>,
    pub default_value: Option<String>,
}
impl DateField {
    pub fn new(
        name: &str,
        not_null: bool,
        key: bool,
        comment: Option<&str>,
        default_value: Option<&str>,
    ) -> Self {
        DateField {
            id: Uuid::new_v4(),
            name: name.to_string(),
            not_null,
            key,
            comment: comment.map(|s| s.to_string()),
            default_value: default_value.map(|s| s.to_string()),
        }
    }
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn default_value(&self) -> Option<&str> {
        self.default_value.as_deref()
    }
    pub fn key(&self) -> bool {
        self.key
    }
    pub fn not_null(&self) -> bool {
        self.not_null
    }
    pub fn extra(&self) -> Option<&str> {
        None
    }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_create_str(&self, kind: String) -> String {
        format!(
            "`{}` {}{}{}{}",
            self.name(),
            kind,
            not_null(self.not_null()),
            default_value(self.default_value(), false),
            comment(self.comment())
        )
    }
    pub fn get_change_str(&self, kind: String, old: &DateField) -> Option<String> {
        if old.name != self.name
            || old.default_value != self.default_value
            || old.not_null != self.not_null
            || old.comment != self.comment
        {
            Some(format!(
                "CHANGE COLUMN `{}` {}",
                old.name,
                self.get_create_str(kind)
            ))
        } else {
            None
        }
    }
}
