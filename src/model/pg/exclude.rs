use super::index::IndexMethod;
use sqlx::{postgres::PgRow, Row};
use std::fmt;
use uuid::Uuid;

use regex::Regex;

#[derive(Clone)]
pub struct Exclude {
    pub id: Uuid,
    pub name: String,
    pub index_method: Option<IndexMethod>,
    pub element: Vec<ExcludeElement>,
    pub comment: Option<String>,
}

impl Exclude {
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn index_method(&self) -> Option<&str> {
        self.index_method.clone().map(|im| im.into())
    }
    pub fn element(&self) -> &Vec<ExcludeElement> {
        &self.element
    }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_create_ddl(&self) -> String {
        format!(
            "CONSTRAINT \"{}\" EXCLUDE ({})",
            self.name(),
            self.element()
                .iter()
                .map(|e| e.get_create_ddl())
                .collect::<Vec<String>>()
                .join(",")
        )
    }
    pub fn get_add_ddl(&self) -> String {
        format!("ADD {}", self.get_create_ddl())
    }
    pub fn get_drop_ddl(&self) -> String {
        format!("DROP CONSTRAINT {}", self.name)
    }
    pub fn get_alter_ddl(
        &self,
        old: &Exclude,
        schema_name: &str,
        table_name: &str,
    ) -> (Vec<String>, Option<String>) {
        let comment = if old.comment() != self.comment() {
            Some(format!(
                "COMMENT ON \"{}\" ON \"{}\".\"{}\" IS '{}'",
                self.name(),
                schema_name,
                table_name,
                self.comment().unwrap_or(""),
            ))
        } else {
            None
        };
        (vec![], comment)
    }
    pub fn get_rename_ddl(&self, other: &Exclude, table_name: &str) -> Vec<String> {
        let mut ddl = Vec::new();
        if other.name != self.name {
            ddl.push(format!(
                "ALTER TABLE {} RENAME CONSTRAINT {} TO {}",
                table_name, other.name, self.name
            ));
        }
        ddl
    }
}
pub fn show_pg_exclude_field(row: &[String]) -> String {
    let mut str = String::from(&row[0]);
    if !row[1].is_empty() {
        str = format!("{} {}", str, row[1]);
    }
    if !&row[2].is_empty() {
        str = format!("{} {}", str, row[2]);
    }
    if !&row[3].is_empty() {
        str = format!("{} \"{}\"", str, row[3]);
    }
    if !&row[4].is_empty() {
        str = format!("{} \"{}\"", str, row[4]);
    }
    if !&row[5].is_empty() {
        str = format!("{} {}", str, row[5]);
    }
    if !&row[6].is_empty() {
        str = format!("{} NULLS {}", str, row[6]);
    }
    str
}
#[derive(Clone)]
pub struct ExcludeElement {
    pub element: String,
    pub operator_class_schema: Option<String>,
    pub operator_class: Option<String>,
    pub order: Option<String>,
    pub nulls_order: Option<String>,
    pub operator_schema: Option<String>,
    pub operator: Option<String>,
}

impl ExcludeElement {
    pub fn element(&self) -> &str {
        self.element.as_str()
    }
    pub fn operator_class_schema(&self) -> Option<&str> {
        self.operator_class_schema.as_deref()
    }
    pub fn operator_class(&self) -> Option<&str> {
        self.operator_class.as_deref()
    }
    pub fn order(&self) -> Option<&str> {
        self.order.as_deref()
    }
    pub fn nulls_order(&self) -> Option<&str> {
        self.nulls_order.as_deref()
    }
    pub fn operator_schema(&self) -> Option<&str> {
        self.operator_schema.as_deref()
    }
    pub fn operator(&self) -> Option<&str> {
        self.operator.as_deref()
    }
    pub fn get_create_ddl(&self) -> String {
        let operator_class =
            if let (Some(ocs), Some(oc)) = (self.operator_class_schema(), self.operator_class()) {
                format!(" \"{ocs}\".\"{oc}\"")
            } else {
                String::from("")
            };

        let operator = if let (Some(os), Some(o)) = (self.operator_schema(), self.operator()) {
            format!(" WITH \"{os}\".{o}")
        } else {
            String::from("")
        };
        format!(
            "\"{}\"{}{}{}{} ",
            self.element(),
            operator_class,
            self.order().unwrap_or(""),
            self.nulls_order().unwrap_or(""),
            operator
        )
    }
}

impl fmt::Display for ExcludeElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}:{}:{}:{}",
            self.element,
            self.operator_class_schema.as_deref().unwrap_or(""),
            self.operator_class.as_deref().unwrap_or(""),
            self.order.as_deref().unwrap_or(""),
            self.nulls_order.as_deref().unwrap_or(""),
            self.operator_schema.as_deref().unwrap_or(""),
            self.operator.as_deref().unwrap_or(""),
        )
    }
}
impl TryFrom<&str> for ExcludeElement {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<ExcludeElement, Self::Error> {
        let arr: Vec<&str> = s.split(':').collect();

        Ok(ExcludeElement {
            element: arr[0].to_string(),
            operator_class_schema: if arr[1].is_empty() {
                None
            } else {
                Some(arr[1].to_string())
            },
            operator_class: if arr[2].is_empty() {
                None
            } else {
                Some(arr[2].to_string())
            },
            order: if arr[3].is_empty() {
                None
            } else {
                Some(arr[3].to_string())
            },
            nulls_order: if arr[4].is_empty() {
                None
            } else {
                Some(arr[4].to_string())
            },
            operator_schema: if arr[5].is_empty() {
                None
            } else {
                Some(arr[5].to_string())
            },
            operator: if arr[6].is_empty() {
                None
            } else {
                Some(arr[6].to_string())
            },
        })
    }
}
pub fn convert_row_to_pg_exclude(rows: Vec<PgRow>) -> Vec<Exclude> {
    rows.iter()
        .map(|row| {
            let def: String = row.try_get("def").unwrap();
            let reg = Regex::new(
                r"EXCLUDE\sUSING\s(?P<index_method>btree|hash|gist|spgist|gin|brin)\s\((?P<element>.+)\)",
            )
            .unwrap();
            let element_reg =
                Regex::new(r"(?P<element>\w+)\s?(?P<order>DESC)?\s?(NULLS\s(?P<nulls_order>FIRST|LAST))?")
                    .unwrap();
            let caps = reg.captures(def.as_str()).unwrap();
            let element = element_reg
                .captures_iter(caps.name("element").unwrap().as_str())
                .map(|cap| ExcludeElement {
                    element: cap.name("element").unwrap().as_str().to_string(),
                    operator_class_schema: None,
                    operator_class: None,
                    order: cap.name("order").map(|c| c.as_str().to_string()),
                    nulls_order: cap.name("nulls_order").map(|c| c.as_str().to_string()),
                    operator_schema: None,
                    operator: None,
                })
                .collect::<Vec<ExcludeElement>>();

            let index_method = caps.name("index_method");
            Exclude {
                id: Uuid::new_v4(),
                name: row.try_get("conname").unwrap(),
                index_method: index_method.map(|im|IndexMethod::try_from(im.as_str()).unwrap()),
                element,
                comment: None,
            }
        })
        .collect()
}
