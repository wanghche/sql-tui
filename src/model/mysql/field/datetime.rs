use super::{comment, default_value, length, not_null, on_update};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct DateTimeField {
    pub id: Uuid,
    pub name: String,
    pub not_null: bool,
    pub key: bool,
    pub comment: Option<String>,
    pub length: Option<String>,
    pub default_value: Option<String>,
    pub on_update: bool,
}
impl DateTimeField {
    pub fn new(
        name: &str,
        not_null: bool,
        key: bool,
        length: Option<&str>,
        comment: Option<&str>,
        default_value: Option<&str>,
        extra: &str,
    ) -> Self {
        DateTimeField {
            id: Uuid::new_v4(),
            name: name.to_string(),
            not_null,
            key,
            comment: comment.map(|s| s.to_string()),
            length: length.map(|s| s.to_string()),
            default_value: default_value.map(|s| s.to_string()),
            on_update: extra.contains("on update"),
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
    pub fn length(&self) -> Option<&str> {
        self.length.as_deref()
    }
    pub fn extra(&self) -> Option<&str> {
        if self.on_update {
            Some("ON UPDATE CURRENT_TIMESTAMP")
        } else {
            None
        }
    }
    pub fn on_update(&self) -> bool {
        self.on_update
    }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_create_str(&self, kind: String) -> String {
        let str = format!("`{}` {}", self.name, kind);
        let str = length(&str, self.length.as_ref());
        let str = not_null(&str, self.not_null);
        let str = default_value(&str, self.default_value.as_ref(), false);
        let str = on_update(&str, self.on_update, self.length.as_ref());
        let str = comment(&str, self.comment.as_ref());
        str
    }
    pub fn get_change_str(&self, kind: String, old: &DateTimeField) -> Option<String> {
        if old.name != self.name
            || old.default_value != self.default_value
            || old.not_null != self.not_null
            || old.length != self.length
            || old.on_update != self.on_update
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
