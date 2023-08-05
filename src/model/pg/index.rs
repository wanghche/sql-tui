use regex::Regex;
use sqlx::{postgres::PgRow, Row};
use std::fmt;
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};
use uuid::Uuid;

#[derive(Display, EnumIter, EnumString, AsRefStr, Clone)]
#[strum(serialize_all = "UPPERCASE")]
pub enum IndexOrder {
    Asc,
    Desc,
}

#[derive(Display, EnumIter, EnumString, AsRefStr, Clone)]
#[strum(serialize_all = "UPPERCASE")]
pub enum IndexKind {
    FullText,
    Normal,
    Spatial,
    Unique,
}

#[derive(Clone)]
pub struct IndexField {
    pub name: String,
    pub collation_schema: Option<String>,
    pub collation: Option<String>,
    pub operator_class_schema: Option<String>,
    pub operator_class: Option<String>,
    pub sort_order: Option<String>,
    pub nulls_order: Option<String>,
}

impl IndexField {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn collation_schema(&self) -> Option<&str> {
        self.collation_schema.as_deref()
    }
    pub fn collation(&self) -> Option<&str> {
        self.collation.as_deref()
    }
    pub fn operator_class_schema(&self) -> Option<&str> {
        self.operator_class_schema.as_deref()
    }
    pub fn operator_class(&self) -> Option<&str> {
        self.operator_class.as_deref()
    }
    pub fn sort_order(&self) -> Option<&str> {
        self.sort_order.as_deref()
    }
    pub fn nulls_order(&self) -> Option<&str> {
        self.nulls_order.as_deref()
    }
    pub fn to_show_string(&self) -> String {
        let mut str = String::from(self.name());
        if let Some(cs) = self.collation_schema() {
            str = format!("{} {}", str, cs);
        }
        if let Some(c) = self.collation() {
            str = format!("{} {}", str, c);
        }
        if let Some(ocs) = self.operator_class_schema() {
            str = format!("{} \"{}\"", str, ocs);
        }
        if let Some(oc) = self.operator_class() {
            str = format!("{} \"{}\"", str, oc);
        }
        if let Some(sort) = self.sort_order() {
            str = format!("{} {}", str, sort);
        }
        if let Some(null) = self.nulls_order() {
            str = format!("{} NULLS {}", str, null);
        }
        str
    }
}

pub fn show_pg_index_field(row: &[String]) -> String {
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

impl fmt::Display for IndexField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}",
            self.name,
            self.collation_schema.as_deref().unwrap_or(""),
            self.collation.as_deref().unwrap_or(""),
            self.operator_class_schema.as_deref().unwrap_or("")
        )
    }
}

impl TryFrom<&str> for IndexField {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<IndexField, Self::Error> {
        let arr: Vec<&str> = s.split(':').collect();

        Ok(IndexField {
            name: arr[0].to_string(),
            collation_schema: if arr[1].is_empty() {
                None
            } else {
                Some(arr[1].to_string())
            },
            collation: if arr[2].is_empty() {
                None
            } else {
                Some(arr[2].to_string())
            },
            operator_class_schema: if arr[3].is_empty() {
                None
            } else {
                Some(arr[3].to_string())
            },
            operator_class: if arr[4].is_empty() {
                None
            } else {
                Some(arr[4].to_string())
            },
            sort_order: if arr[5].is_empty() {
                None
            } else {
                Some(arr[5].to_string())
            },
            nulls_order: if arr[6].is_empty() {
                None
            } else {
                Some(arr[6].to_string())
            },
        })
    }
}
#[derive(Display, EnumIter, EnumString, AsRefStr, IntoStaticStr, Clone)]
#[strum(serialize_all = "lowercase")]
pub enum IndexMethod {
    Btree,
    Hash,
    Gist,
    Gin,
    SPGist,
    Brin,
}

#[derive(Clone)]
pub struct Index {
    pub id: Uuid,
    pub name: String,
    pub fields: Vec<IndexField>,
    pub index_method: Option<IndexMethod>,
    pub unique: bool,
    pub concurrent: bool,
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
    pub fn index_method(&self) -> Option<&IndexMethod> {
        self.index_method.as_ref()
    }
    pub fn unique(&self) -> bool {
        self.unique
    }
    pub fn concurrent(&self) -> bool {
        self.concurrent
    }
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    pub fn get_create_ddl(&self, schema_name: &str, table_name: &str) -> (String, Option<String>) {
        let index_ddl = format!(
            "CREATE {}INDEX{} \"{}\" ON \"{}\".\"{}\" {}(\n{}\n);",
            if self.unique { "UNIQUE " } else { "" },
            if self.concurrent { " CONCURRENT" } else { "" },
            self.name,
            schema_name,
            table_name,
            if let Some(m) = self.index_method() {
                format!("USING {} ", m)
            } else {
                String::from("")
            },
            self.fields
                .iter()
                .map(|f| {
                    let collate = if let (Some(s), Some(c)) =
                        (f.collation_schema.as_deref(), f.collation.as_deref())
                    {
                        format!(" COLLATE \"{}\".\"{}\"", s, c)
                    } else {
                        String::from("")
                    };
                    let opclass = if let (Some(s), Some(c)) = (
                        f.operator_class_schema.as_deref(),
                        f.operator_class.as_deref(),
                    ) {
                        format!(" \"{}\".\"{}\"", s, c)
                    } else {
                        String::from("")
                    };

                    format!(
                        "  {}{}{}{}{}",
                        f.name,
                        collate,
                        opclass,
                        if let Some(s) = f.sort_order.as_deref() {
                            format!(" {}", s)
                        } else {
                            String::from("")
                        },
                        if let Some(n) = f.nulls_order.as_deref() {
                            format!(" {}", n)
                        } else {
                            String::from("")
                        }
                    )
                })
                .collect::<Vec<String>>()
                .join(",")
        );
        let comment_ddl = self.comment().map(|comment| {
            format!(
                "COMMENT ON INDEX \"{}\".\"{}\" IS '{}';",
                schema_name,
                self.name(),
                comment
            )
        });
        (index_ddl, comment_ddl)
    }
    pub fn get_add_ddl(&self, schema_name: &str, table_name: &str) -> (String, Option<String>) {
        self.get_create_ddl(schema_name, table_name)
    }
    pub fn get_rename_ddl(&self, old: &Index) -> Vec<String> {
        let mut ddl = Vec::new();
        if old.name != self.name {
            ddl.push(format!(
                "ALTER INDEX \"{}\" RENAME TO \"{}\"",
                old.name, self.name
            ));
        }
        ddl
    }
    pub fn get_drop_ddl(&self) -> String {
        format!("DROP INDEX {}", self.name)
    }
    pub fn get_alter_ddl(&self, old: &Index, schema_name: &str) -> (Vec<String>, Option<String>) {
        let comment_ddl = if old.comment() != self.comment() {
            Some(format!(
                "COMMENT ON INDEX \"{}\".\"{}\" IS '{}';",
                schema_name,
                self.name(),
                self.comment().unwrap_or("")
            ))
        } else {
            None
        };
        (vec![], comment_ddl)
    }
}
pub fn convert_show_index_to_pg_indexes(rows: Vec<PgRow>) -> Vec<Index> {
    rows.iter()
        .map(|row| {
            let def = row.try_get::<String, _>("indexdef").unwrap().replace("\"","");
            let create_regex = Regex::new(r"CREATE\s(?P<u>UNIQUE\s)?INDEX\s(?P<c>CONCURRENTLY\s)?(?:\w+)\sON\s(?:\w+).(?:\w+)\sUSING\s(?P<m>btree|hash|gist|spgist|gin|brin)\s\((?P<index>.+)\)")
                .unwrap();
            let create_captures = create_regex.captures(def.as_str()).unwrap();
            let fields_regex = Regex::new(r"(?P<name>\w+)\s?(COLLATE\s(?P<collate>\w+))?\s?(?P<sort>DESC)?\s?(NULLS\s(?P<null>FIRST|LAST))?").unwrap();
            let fields = fields_regex.captures_iter(create_captures.name("index").unwrap().as_str()).map(|cap| IndexField
                {
                   name: cap.name("name").unwrap().as_str().to_string(), 
                   collation_schema: None,
                   collation: None,
                   operator_class: None,
                   operator_class_schema: None,
                   sort_order: cap.name("sort").map(|s| s.as_str().to_string()) ,
                   nulls_order: cap.name("null").map(|s| s.as_str().to_string()),

                }).collect::<Vec<IndexField>>();
            Index {
                id: Uuid::new_v4(),
                name: row.try_get("indexname").unwrap(),
                fields ,
                index_method: create_captures.name("m").map(|method| IndexMethod::try_from(method.as_str()).unwrap()),
                unique: create_captures.name("u").is_some(),
                concurrent: create_captures.name("c").is_some(),
                comment: row.try_get("comment").unwrap(),
            }
        })
        .collect::<Vec<Index>>()
}
