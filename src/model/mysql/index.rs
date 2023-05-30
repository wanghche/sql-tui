use itertools::Itertools;
use sqlx::{mysql::MySqlRow, Row};
use std::fmt;
use strum::{AsRefStr, Display, EnumIter, EnumString};
use uuid::Uuid;

#[derive(Display, EnumIter, EnumString, AsRefStr, Clone, PartialEq)]
#[strum(serialize_all = "UPPERCASE")]
pub enum IndexOrder {
    Asc,
    Desc,
}

#[derive(Clone, PartialEq)]
pub struct IndexField {
    pub name: String,
    pub sub_part: Option<i64>,
    pub order: Option<IndexOrder>,
}

impl fmt::Display for IndexField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut str = format!("`{}`", self.name);
        if let Some(sub_part) = self.sub_part.as_ref() {
            str = format!("{}({})", str, sub_part);
        }
        if let Some(order) = self.order.as_ref() {
            str = format!("{} {}", str, order);
        }
        write!(f, "{}", str)
    }
}

impl TryFrom<&str> for IndexField {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<IndexField, Self::Error> {
        let arr: Vec<&str> = s.split(':').collect();

        Ok(IndexField {
            name: arr.first().unwrap().to_string(),
            sub_part: arr.get(1).unwrap().to_owned().parse::<i64>().ok(),
            order: if let Some(order) = arr.get(2) {
                IndexOrder::try_from(*order).ok()
            } else {
                None
            },
        })
    }
}

#[derive(Display, EnumIter, EnumString, AsRefStr, Clone, PartialEq)]
#[strum(serialize_all = "UPPERCASE")]
pub enum IndexKind {
    FullText,
    Normal,
    Spatial,
    Unique,
}

#[derive(Display, EnumIter, EnumString, AsRefStr, Clone, PartialEq)]
#[strum(serialize_all = "UPPERCASE")]
pub enum IndexMethod {
    Btree,
    Hash,
}

#[derive(Clone)]
pub struct Index {
    pub id: Uuid,
    pub name: String,
    pub fields: Vec<IndexField>,
    pub kind: IndexKind,
    pub method: Option<IndexMethod>,
    pub comment: Option<String>,
}
impl Index {
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn fields(&self) -> &Vec<IndexField> {
        &self.fields
    }
    pub fn kind(&self) -> &IndexKind {
        &self.kind
    }
    pub fn method(&self) -> Option<&IndexMethod> {
        self.method.as_ref()
    }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_create_ddl(&self) -> String {
        let fields: Vec<String> = self.fields.iter().map(|s| s.to_string()).collect();
        let mut sql = match self.kind {
            IndexKind::FullText => {
                format!(
                    "FULLTEXT INDEX `{}`({}){}",
                    self.name,
                    fields.join(","),
                    if let Some(m) = self.method.as_ref() {
                        format!(" USING {}", m)
                    } else {
                        "".to_string()
                    },
                )
            }
            IndexKind::Normal => {
                format!(
                    "INDEX `{}`({}){}",
                    self.name,
                    fields.join(","),
                    if let Some(m) = self.method.as_ref() {
                        format!(" USING {}", m)
                    } else {
                        "".to_string()
                    },
                )
            }
            IndexKind::Spatial => format!(
                "SPATIAL INDEX `{}`({}) USING {}",
                self.name,
                fields.join(","),
                if let Some(m) = self.method.as_ref() {
                    format!(" USING {}", m)
                } else {
                    "".to_string()
                },
            ),
            IndexKind::Unique => format!(
                "UNIQUE INDEX `{}`({}){}",
                self.name,
                fields.join(","),
                if let Some(m) = self.method.as_ref() {
                    format!(" USING {}", m)
                } else {
                    "".to_string()
                },
            ),
        };

        if let Some(comment) = self.comment().as_ref() {
            sql = format!("{} COMMENT '{}'", sql, comment);
        }
        sql
    }
    pub fn get_drop_ddl(&self) -> String {
        format!("DROP INDEX {}", self.name)
    }
    pub fn get_add_ddl(&self) -> String {
        format!("ADD {}", self.get_create_ddl())
    }
    pub fn get_alter_ddl(&self, old: &Index) -> Vec<String> {
        let mut ddl = Vec::new();
        if old.name != self.name {
            ddl.push(format!("RENAME INDEX {} TO {}", old.name, self.name))
        }
        if old.fields != self.fields
            || old.kind != self.kind
            || old.method != self.method
            || old.comment != self.comment
        {
            if old.name == self.name {
                ddl.push(format!("DROP INDEX `{}`", self.name));
            }
            ddl.push(self.get_add_ddl());
        }
        ddl
    }
}
pub fn convert_show_index_to_mysql_indexes(fields: Vec<MySqlRow>) -> Vec<Index> {
    let names = fields
        .iter()
        .map(|f| f.try_get("Key_name").unwrap())
        .collect::<Vec<String>>();
    names
        .into_iter()
        .unique()
        .map(|name| {
            let index_fields = fields
                .iter()
                .filter(|f| f.try_get::<String, _>("Key_name").unwrap() == name)
                .map(|f| {
                    let order: Option<String> = f.try_get("Collation").unwrap();
                    let order = if let Some(o) = order {
                        if o == "A" {
                            Some(IndexOrder::Asc)
                        } else if o == "D" {
                            Some(IndexOrder::Desc)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    IndexField {
                        name: f.try_get("Column_name").unwrap(),
                        sub_part: f.try_get("Sub_part").unwrap(),
                        order,
                    }
                })
                .collect();

            let row = fields
                .iter()
                .find(|f| f.try_get::<String, _>("Key_name").unwrap() == name)
                .unwrap();

            let index_type = row.try_get::<String, _>("Index_type").unwrap();

            let kind = if let Ok(m) = IndexKind::try_from(index_type.as_str()) {
                m
            } else if row.try_get::<i32, _>("Non_unique").unwrap() == 0 {
                IndexKind::Unique
            } else {
                IndexKind::Normal
            };
            Index {
                id: Uuid::new_v4(),
                name,
                fields: index_fields,
                method: IndexMethod::try_from(index_type.as_str()).ok(),
                kind,
                comment: None,
            }
        })
        .collect()
}
pub fn show_mysql_index_field(row: &[String]) -> String {
    let mut str = String::from(&row[0]);
    if !row[1].is_empty() {
        str.push(' ');
        str.push_str(&row[1]);
    }
    if let Some(order) = row.get(2) {
        str.push(' ');
        str.push_str(order);
    }
    str
}
