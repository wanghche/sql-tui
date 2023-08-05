use super::{comment, default_value, length_decimal, not_null, unsigned, zerofill};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct DecimalField {
    pub id: Uuid,
    pub name: String,
    pub not_null: bool,
    pub key: bool,
    pub comment: Option<String>,
    pub length: Option<String>,
    pub decimal: Option<String>,
    pub default_value: Option<String>,
    pub unsigned: bool,
    pub zerofill: bool,
}

impl DecimalField {
    pub fn new(
        name: &str,
        not_null: bool,
        key: bool,
        length: &str,
        decimal: &str,
        unsigned: bool,
        zerofill: bool,
        comment: Option<&str>,
        default_value: Option<&str>,
    ) -> Self {
        DecimalField {
            id: Uuid::new_v4(),
            name: name.to_string(),
            not_null,
            key,
            comment: comment.map(|s| s.to_string()),
            length: Some(length.to_string()),
            decimal: Some(decimal.to_string()),
            default_value: default_value.map(|s| s.to_string()),
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
    pub fn length(&self) -> Option<&str> {
        self.length.as_deref()
    }
    pub fn decimal(&self) -> Option<&str> {
        self.decimal.as_deref()
    }
    pub fn unsigned(&self) -> bool {
        self.unsigned
    }
    pub fn zerofill(&self) -> bool {
        self.zerofill
    }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_create_str(&self, kind: String) -> String {
        format!(
            "`{}` {}{}{}{}{}{}{}",
            self.name(),
            kind,
            length_decimal(self.length(), self.decimal()),
            unsigned(self.unsigned()),
            zerofill(self.zerofill()),
            not_null(self.not_null()),
            default_value(self.default_value(), false),
            comment(self.comment())
        )
    }
    pub fn get_change_str(&self, kind: String, old: &DecimalField) -> Option<String> {
        if old.name != self.name
            || old.default_value != self.default_value
            || old.not_null != self.not_null
            || old.length != self.length
            || old.decimal != self.decimal
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
