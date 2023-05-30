use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{
    postgres::{PgColumn, PgPool, PgRow},
    Column, Postgres, Row, TypeInfo,
};
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};
use time::{Date, Time};
use uuid::Uuid;

#[derive(Default, EnumIter, Display, AsRefStr, EnumString, Clone, IntoStaticStr, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum FieldKind {
    BigSerial,
    Bit,
    Bool,
    Box,
    Bytea,
    Char,
    Cidr,
    Circle,
    Date,
    Decimal,
    Float4,
    Float8,
    Inet,
    Int2,
    #[default]
    Int4,
    Int8,
    Interval,
    Json,
    Jsonb,
    Line,
    Lseg,
    Macaddr,
    Money,
    Numeric,
    Path,
    Point,
    Polygon,
    Serial,
    Serial2,
    Serial4,
    Serial8,
    SmallSerial,
    Text,
    Time,
    Timestamp,
    TimestampTz,
    TimeTz,
    TsQuery,
    TsVector,
    Uuid,
    VarBit,
    VarChar,
    Xml,
}

#[derive(Clone)]
pub struct Field {
    pub id: Uuid,
    pub name: String,
    pub kind: FieldKind,
    pub not_null: bool,
    pub key: bool,
    pub comment: Option<String>,
    pub default_value: Option<String>,
    pub length: Option<i32>,
    pub decimal: Option<i32>,
}

impl Field {
    pub fn id(&self) -> &Uuid {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn kind(&self) -> &FieldKind {
        &self.kind
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
    pub fn length(&self) -> Option<String> {
        self.length.map(|l| l.to_string())
    }
    pub fn decimal(&self) -> Option<String> {
        self.decimal.map(|d| d.to_string())
    }
    fn get_kind_ddl(&self) -> String {
        match self.kind {
            FieldKind::VarChar
            | FieldKind::Char
            | FieldKind::Interval
            | FieldKind::Time
            | FieldKind::Timestamp
            | FieldKind::TimestampTz
            | FieldKind::TimeTz
            | FieldKind::VarBit
            | FieldKind::Bit => {
                if let Some(l) = self.length() {
                    format!("{}({})", self.kind.to_string(), l)
                } else {
                    self.kind.to_string()
                }
            }
            FieldKind::Decimal | FieldKind::Numeric => {
                if let Some(l) = self.length() {
                    if let Some(d) = self.decimal() {
                        format!("{}({},{})", self.kind.to_string(), l, d)
                    } else {
                        format!("{}({})", self.kind.to_string(), l)
                    }
                } else {
                    self.kind.to_string()
                }
            }
            _ => self.kind.to_string(),
        }
    }
    pub fn get_create_ddl(&self, schema_name: &str, table_name: &str) -> (String, Option<String>) {
        let field_ddl = format!(
            "  \"{}\" {} {}{}",
            self.name,
            self.get_kind_ddl(),
            if self.not_null { "NOT NULL" } else { "" },
            if let Some(de_val) = self.default_value() {
                format!(r#" DEFAULT {}"#, de_val)
            } else {
                String::from("")
            }
        );

        let comment_ddl = if let Some(comment) = self.comment() {
            Some(format!(
                "COMMENT ON COLUMN \"{}\".\"{}\".\"{}\" IS '{}';",
                schema_name,
                table_name,
                self.name(),
                comment
            ))
        } else {
            None
        };
        (field_ddl, comment_ddl)
    }
    pub fn get_drop_ddl(&self) -> String {
        format!("DROP COLUMN {}", self.name)
    }
    pub fn get_add_ddl(&self, schema_name: &str, table_name: &str) -> (String, Option<String>) {
        let (field_ddl, comment_ddl) = self.get_create_ddl(schema_name, table_name);
        (format!("ADD COLUMN {}", field_ddl), comment_ddl)
    }
    pub fn get_alter_ddl(
        &self,
        old: &Field,
        schema_name: &str,
        table_name: &str,
    ) -> (Vec<String>, Option<String>) {
        let mut ddl = Vec::new();
        if old.kind != self.kind || old.length != self.length || old.decimal != self.decimal {
            ddl.push(format!(
                "ALTER COLUMN \"{}\" TYPE {}",
                self.name,
                self.get_kind_ddl()
            ));
        }
        if old.default_value != self.default_value {
            if let Some(dv) = self.default_value() {
                ddl.push(format!("ALTER COLUMN \"{}\" SET DEFAULT {}", self.name, dv));
            } else {
                ddl.push(format!("ALTER COLUMN \"{}\" DROP DEFAULT", self.name));
            }
        }
        if old.not_null != self.not_null {
            ddl.push(format!(
                "ALTER COLUMN \"{}\" {} NOT NULL",
                self.name,
                if self.not_null { "SET" } else { "DROP" }
            ));
        }
        let comment_ddl = if old.comment != self.comment {
            Some(format!(
                "COMMENT ON COLUMN \"{}\".\"{}\".\"{}\" IS '{}';",
                schema_name,
                table_name,
                self.name(),
                self.comment().unwrap_or("")
            ))
        } else {
            None
        };
        (ddl, comment_ddl)
    }
    pub fn get_rename_ddl(&self, schema_name: &str, table_name: &str, old: &Field) -> Vec<String> {
        let mut ddl = Vec::new();
        if old.name != self.name {
            ddl.push(format!(
                "ALTER TABLE \"{}\".\"{}\" RENAME COLUMN {} TO {};",
                schema_name, table_name, old.name, self.name
            ));
        }
        ddl
    }
}

pub async fn get_pg_field_names(pool: &PgPool, schema: &str, table: &str) -> Result<Vec<String>> {
    let fields: Vec<String> = sqlx::query(format!("select column_name from information_schema.columns where table_schema = '{}' and table_name = '{}'",schema, table).as_str())
        .fetch_all(pool)
        .await?
        .iter()
        .map(|t| t.try_get("column_name").unwrap())
        .collect();
    Ok(fields)
}
pub fn get_pg_field_value(field: &Field, row: &PgRow) -> String {
    fn get_value<'r, T>(field: &Field, row: &'r PgRow) -> String
    where
        T: std::fmt::Display + sqlx::Decode<'r, Postgres> + sqlx::Type<Postgres>,
    {
        let col_name = field.name();
        if !field.not_null() {
            let i: Option<T> = row.try_get(col_name).unwrap();
            i.map(|i| i.to_string()).unwrap_or_default()
        } else {
            let i: T = row.try_get(col_name).unwrap();
            i.to_string()
        }
    }

    match field.kind() {
        FieldKind::VarChar | FieldKind::Char | FieldKind::Text => get_value::<String>(field, row),
        FieldKind::Int2 | FieldKind::Serial2 | FieldKind::SmallSerial => {
            get_value::<i16>(field, row)
        }
        FieldKind::Int4 | FieldKind::Serial4 | FieldKind::Serial => get_value::<i32>(field, row),
        FieldKind::Int8 | FieldKind::Serial8 | FieldKind::BigSerial => get_value::<i64>(field, row),
        FieldKind::Numeric | FieldKind::Decimal => get_value::<f32>(field, row),
        FieldKind::Float4 => get_value::<f32>(field, row),
        FieldKind::Float8 | FieldKind::Money => get_value::<f64>(field, row),
        FieldKind::Bit | FieldKind::VarBit => "Bit".to_string(),
        FieldKind::Json | FieldKind::Jsonb => "Json".to_string(),
        FieldKind::Point => "Point".to_string(),
        FieldKind::Polygon => "Polygon".to_string(),
        FieldKind::Time | FieldKind::TimeTz => get_value::<Time>(field, row),
        FieldKind::Timestamp | FieldKind::TimestampTz => get_value::<DateTime<Utc>>(field, row),
        FieldKind::Bool => get_value::<bool>(field, row),
        FieldKind::Box => "Box".to_string(),
        FieldKind::Bytea => "Bytea".to_string(),
        FieldKind::Cidr | FieldKind::Inet | FieldKind::Macaddr => get_value::<String>(field, row),
        FieldKind::Date => get_value::<Date>(field, row),
        FieldKind::Circle => "Circle".to_string(),
        FieldKind::Interval => "Interval".to_string(),
        FieldKind::Line => "Line".to_string(),
        FieldKind::Lseg => "Lseg".to_string(),
        FieldKind::Path => "Path".to_string(),
        FieldKind::TsQuery => "TsQuery".to_string(),
        FieldKind::TsVector => "TsVector".to_string(),
        FieldKind::Uuid => "Uuid".to_string(),
        FieldKind::Xml => "Xml".to_string(),
    }
}

pub fn get_pg_column_value(column: &PgColumn, row: &PgRow) -> String {
    let col_name = column.name();
    fn get_numeric<'r, T>(col_name: &str, row: &'r PgRow) -> String
    where
        T: Default + std::fmt::Display + sqlx::Decode<'r, Postgres> + sqlx::Type<Postgres>,
    {
        let i: Option<T> = row.try_get(col_name).unwrap();
        i.unwrap_or_default().to_string()
    }

    match column.type_info().name() {
        "VARCHAR" | "CHAR" | "TEXT" => {
            let value: Option<String> = row.try_get(col_name).unwrap();
            value.unwrap_or_default()
        }
        "INT2" => get_numeric::<i16>(col_name, row),
        "INT4" => get_numeric::<i32>(col_name, row),
        "INT8" => get_numeric::<i64>(col_name, row),
        "NUMERIC" | "DECIMAL" | "FLOAT4" => get_numeric::<f32>(col_name, row),
        "FLOAT8" => get_numeric::<f64>(col_name, row),
        "DATE" => "Date".to_string(),
        "TIME" => "Time".to_string(),
        _ => {
            println!("{}", column.type_info().name());
            String::new()
        }
    }
}

pub fn convert_show_column_to_pg_fields(fields: Vec<PgRow>, key_names: Vec<String>) -> Vec<Field> {
    fields
        .iter()
        .map(|r| {
            let name: String = r.try_get("column_name").unwrap();
            let key = key_names.contains(&name);

            let kind =
                FieldKind::try_from(r.try_get::<String, _>("udt_name").unwrap().as_str()).unwrap();

            let length = match kind {
                FieldKind::Int4 | FieldKind::Int2 | FieldKind::Decimal | FieldKind::Numeric => {
                    Some(r.try_get("numeric_precision").unwrap())
                }
                FieldKind::VarChar | FieldKind::Char => {
                    r.try_get("character_maximum_length").unwrap()
                }
                _ => None,
            };
            let decimal = match kind {
                FieldKind::Decimal | FieldKind::Numeric => {
                    Some(r.try_get("numeric_scale").unwrap())
                }
                _ => None,
            };
            Field {
                id: Uuid::new_v4(),
                name,
                kind,
                not_null: if r.try_get::<String, _>("is_nullable").unwrap() == "YES" {
                    false
                } else {
                    true
                },
                key,
                comment: r.try_get("comment").unwrap(),
                default_value: r.try_get("column_default").unwrap(),
                length,
                decimal,
            }
        })
        .collect::<Vec<Field>>()
}
