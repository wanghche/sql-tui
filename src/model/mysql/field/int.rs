use super::{auto_increment, comment, default_value, length, not_null, unsigned, zerofill};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct IntField {
    pub id: Uuid,
    pub name: String,
    pub not_null: bool,
    pub key: bool,
    pub comment: Option<String>,
    pub length: Option<String>,
    pub default_value: Option<String>,
    pub auto_increment: bool,
    pub unsigned: bool,
    pub zerofill: bool,
}

impl IntField {
    pub fn new(
        name: &str,
        not_null: bool,
        key: bool,
        length: Option<&str>,
        default_value: Option<&str>,
        unsigned: bool,
        zerofill: bool,
        comment: Option<&str>,
        extra: &str,
    ) -> Self {
        IntField {
            id: Uuid::new_v4(),
            name: name.to_string(),
            not_null,
            key,
            comment: comment.map(|s| s.to_string()),
            length: length.map(|s| s.to_string()),
            default_value: default_value.map(|s| s.to_string()),
            auto_increment: extra.contains("auto_increment"),
            unsigned,
            zerofill,
        }
    }
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn not_null(&self) -> bool {
        self.not_null
    }
    pub fn key(&self) -> bool {
        self.key
    }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn default_value(&self) -> Option<&str> {
        self.default_value.as_deref()
    }
    pub fn length(&self) -> Option<&str> {
        self.length.as_deref()
    }
    pub fn auto_increment(&self) -> bool {
        self.auto_increment
    }
    pub fn unsigned(&self) -> bool {
        self.unsigned
    }
    pub fn zerofill(&self) -> bool {
        self.zerofill
    }
    pub fn extra(&self) -> Option<&str> {
        if self.auto_increment {
            Some("AUTO_INCREMENT")
        } else {
            None
        }
    }
    pub fn get_create_str(&self, kind: String) -> String {
        format!(
            "`{}` {}{}{}{}{}{}{}{}",
            self.name(),
            kind,
            length(self.length()),
            unsigned(self.unsigned()),
            zerofill(self.zerofill()),
            not_null(self.not_null()),
            default_value(self.default_value(), false),
            auto_increment(self.auto_increment()),
            comment(self.comment())
        )
    }
    pub fn get_change_str(&self, kind: String, old: &IntField) -> Option<String> {
        if old.name != self.name
            || old.default_value != self.default_value
            || old.not_null != self.not_null
            || old.length != self.length
            || old.auto_increment != self.auto_increment
            || old.unsigned != self.unsigned
            || old.zerofill != self.zerofill
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
