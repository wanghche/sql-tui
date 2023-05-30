use crate::model::mysql::FieldKind;

#[derive(Clone)]
pub struct MySQLColumn {
    pub name: String,
    pub kind: FieldKind,
    pub unsigned: bool,
    pub nullable: bool,
    pub key: String,
    pub default: Option<String>,
    pub extra: String,
}
