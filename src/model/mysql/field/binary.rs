use super::{comment, default_value, length, not_null};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct BinaryField {
    pub id: Uuid,
    pub name: String,
    pub not_null: bool,
    pub key: bool,
    pub comment: Option<String>,
    pub length: String,
    pub default_value: Option<String>,
}

impl BinaryField {
    pub fn new(
        name: &str,
        not_null: bool,
        key: bool,
        length: &str,
        comment: Option<&str>,
        default_value: Option<&str>,
    ) -> Self {
        BinaryField {
            id: Uuid::new_v4(),
            name: name.to_string(),
            not_null,
            key,
            length: length.to_string(),
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
    pub fn key(&self) -> bool {
        self.key
    }
    pub fn default_value(&self) -> Option<&str> {
        self.default_value.as_deref()
    }
    pub fn not_null(&self) -> bool {
        self.not_null
    }
    pub fn length(&self) -> &str {
        &self.length
    }
    pub fn extra(&self) -> Option<&str> {
        None
    }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_create_str(&self, kind: String) -> String {
        format!(
            "`{}` {}{}{}{}{}",
            self.name(),
            kind,
            length(Some(self.length())),
            not_null(self.not_null),
            default_value(self.default_value(), false),
            comment(self.comment())
        )
    }
    pub fn get_change_str(&self, kind: String, old: &BinaryField) -> Option<String> {
        if old.name != self.name
            || old.default_value != self.default_value
            || old.not_null != self.not_null
            || old.length != self.length
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
