use super::{character_set, collation, comment, not_null};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TextField {
    pub id: Uuid,
    pub name: String,
    pub not_null: bool,
    pub key: bool,
    pub comment: Option<String>,
    pub character_set: Option<String>,
    pub collation: Option<String>,
}
impl TextField {
    pub fn new(
        name: &str,
        not_null: bool,
        key: bool,
        collation: &str,
        comment: Option<&str>,
    ) -> Self {
        let underline_index = collation.find('_').unwrap();
        let charset = &collation[..underline_index];
        TextField {
            id: Uuid::new_v4(),
            name: name.to_string(),
            not_null,
            key,
            comment: comment.map(|s| s.to_string()),
            character_set: Some(charset.to_string()),
            collation: Some(collation.to_string()),
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
    pub fn character_set(&self) -> Option<&str> {
        self.character_set.as_deref()
    }
    pub fn collation(&self) -> Option<&str> {
        self.collation.as_deref()
    }
    pub fn extra(&self) -> Option<&str> {
        None
    }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    pub fn get_create_str(&self, kind: String) -> String {
        let str = format!("`{}` {}", self.name, kind);
        let str = character_set(&str, self.character_set.as_deref());
        let str = collation(&str, self.collation.as_deref());
        let str = not_null(&str, self.not_null);
        let str = comment(&str, self.comment.as_ref());
        str
    }
    pub fn get_change_str(&self, kind: String, old: &TextField) -> Option<String> {
        if old.name != self.name
            || old.not_null != self.not_null
            || old.comment != self.comment
            || old.character_set != self.character_set
            || old.collation != self.collation
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
