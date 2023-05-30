use super::{comment, not_null};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct SimpleField {
    pub id: Uuid,
    pub name: String,
    pub not_null: bool,
    pub key: bool,
    pub comment: Option<String>,
}
impl SimpleField {
    pub fn new(name: &str, not_null: bool, key: bool, comment: Option<&str>) -> Self {
        SimpleField {
            id: Uuid::new_v4(),
            name: name.to_string(),
            not_null,
            key,
            comment: comment.map(|s| s.to_string()),
        }
    }
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn default_value(&self) -> Option<&str> {
        None
    }
    pub fn not_null(&self) -> bool {
        self.not_null
    }
    pub fn key(&self) -> bool {
        self.key
    }
    pub fn extra(&self) -> Option<&str> {
        None
    }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_create_str(&self, kind: String) -> String {
        let str = format!("`{}` {}", self.name, kind);
        let str = not_null(&str, self.not_null);
        let str = comment(&str, self.comment.as_ref());
        str
    }
    pub fn get_change_str(&self, kind: String, old: &SimpleField) -> Option<String> {
        if old.name != self.name || old.not_null != self.not_null || old.comment != self.comment {
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
