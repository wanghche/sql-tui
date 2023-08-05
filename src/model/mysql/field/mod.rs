use anyhow::Result;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use regex::Regex;
use sqlx::{
    mysql::{MySqlColumn, MySqlRow},
    types::{BigDecimal, JsonValue},
    Column, MySql, MySqlPool, Row, TypeInfo,
};
use strum::{AsRefStr, Display, EnumIter, EnumString};
use uuid::Uuid;

pub use self::{
    binary::*, char::*, date::*, datetime::*, decimal::*, enumeration::*, float::*, int::*,
    simple::*, text::*, time::*,
};

mod binary;
mod char;
mod date;
mod datetime;
mod decimal;
mod enumeration;
mod float;
mod int;
mod simple;
mod text;
mod time;

#[derive(Debug, AsRefStr, Display, EnumString, Clone, PartialEq, Eq, EnumIter)]
#[strum(serialize_all = "lowercase")]
pub enum FieldKind {
    BigInt,
    Binary,
    Bit,
    Blob,
    Char,
    Date,
    DateTime,
    Decimal,
    Double,
    Enum,
    Float,
    Geometry,
    #[strum(serialize = "geomcollection")]
    GeometryCollection,
    Int,
    Integer,
    Json,
    LineString,
    LongBlob,
    LongText,
    MediumBlob,
    MediumInt,
    MediumText,
    MultiLineString,
    MultiPoint,
    MultiPolygon,
    Numeric,
    Point,
    Polygon,
    Real,
    Set,
    SmallInt,
    Text,
    Time,
    Timestamp,
    TinyBlob,
    TinyInt,
    TinyText,
    VarBinary,
    VarChar,
    Year,
}

#[derive(Clone, AsRefStr, Display, Debug)]
#[strum(serialize_all = "lowercase")]
pub enum Field {
    BigInt(IntField),
    Binary(BinaryField),
    Bit(BinaryField),
    Blob(SimpleField),
    Char(CharField),
    Date(DateField),
    DateTime(DateTimeField),
    Decimal(DecimalField),
    Double(FloatField),
    Enum(EnumField),
    Float(FloatField),
    Geometry(SimpleField),
    GeometryCollection(SimpleField),
    Int(IntField),
    Integer(IntField),
    Json(SimpleField),
    LineString(SimpleField),
    LongBlob(SimpleField),
    LongText(TextField),
    MediumBlob(SimpleField),
    MediumInt(IntField),
    MediumText(TextField),
    MultiLineString(SimpleField),
    MultiPoint(SimpleField),
    MultiPolygon(SimpleField),
    Numeric(DecimalField),
    Point(SimpleField),
    Polygon(SimpleField),
    Real(FloatField),
    Set(EnumField),
    SmallInt(IntField),
    Text(TextField),
    Time(TimeField),
    Timestamp(DateTimeField),
    TinyBlob(SimpleField),
    TinyInt(IntField),
    TinyText(TextField),
    VarBinary(BinaryField),
    VarChar(CharField),
    Year(DateField),
}

impl Field {
    pub fn id(&self) -> &Uuid {
        match self {
            Field::BigInt(i) => i.id(),
            Field::Binary(b) => b.id(),
            Field::Bit(b) => b.id(),
            Field::Blob(s) => s.id(),
            Field::Char(c) => c.id(),
            Field::Date(d) => d.id(),
            Field::DateTime(dt) => dt.id(),
            Field::Decimal(d) => d.id(),
            Field::Double(f) => f.id(),
            Field::Enum(e) => e.id(),
            Field::Float(f) => f.id(),
            Field::Geometry(g) => g.id(),
            Field::GeometryCollection(g) => g.id(),
            Field::Int(i) => i.id(),
            Field::Integer(i) => i.id(),
            Field::Json(s) => s.id(),
            Field::LineString(s) => s.id(),
            Field::LongBlob(s) => s.id(),
            Field::LongText(lt) => lt.id(),
            Field::MediumBlob(m) => m.id(),
            Field::MediumInt(i) => i.id(),
            Field::MediumText(t) => t.id(),
            Field::MultiLineString(s) => s.id(),
            Field::MultiPoint(s) => s.id(),
            Field::MultiPolygon(s) => s.id(),
            Field::Numeric(d) => d.id(),
            Field::Point(s) => s.id(),
            Field::Polygon(s) => s.id(),
            Field::Real(f) => f.id(),
            Field::Set(e) => e.id(),
            Field::SmallInt(i) => i.id(),
            Field::Text(t) => t.id(),
            Field::Time(t) => t.id(),
            Field::Timestamp(d) => d.id(),
            Field::TinyBlob(s) => s.id(),
            Field::TinyInt(i) => i.id(),
            Field::TinyText(t) => t.id(),
            Field::VarBinary(b) => b.id(),
            Field::VarChar(c) => c.id(),
            Field::Year(d) => d.id(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Field::BigInt(i) => i.name(),
            Field::Binary(b) => b.name(),
            Field::Bit(b) => b.name(),
            Field::Blob(s) => s.name(),
            Field::Char(c) => c.name(),
            Field::Date(d) => d.name(),
            Field::DateTime(dt) => dt.name(),
            Field::Decimal(d) => d.name(),
            Field::Double(f) => f.name(),
            Field::Enum(e) => e.name(),
            Field::Float(f) => f.name(),
            Field::Geometry(g) => g.name(),
            Field::GeometryCollection(g) => g.name(),
            Field::Int(i) => i.name(),
            Field::Integer(i) => i.name(),
            Field::Json(s) => s.name(),
            Field::LineString(s) => s.name(),
            Field::LongBlob(s) => s.name(),
            Field::LongText(lt) => lt.name(),
            Field::MediumBlob(m) => m.name(),
            Field::MediumInt(i) => i.name(),
            Field::MediumText(t) => t.name(),
            Field::MultiLineString(s) => s.name(),
            Field::MultiPoint(s) => s.name(),
            Field::MultiPolygon(s) => s.name(),
            Field::Numeric(d) => d.name(),
            Field::Point(s) => s.name(),
            Field::Polygon(s) => s.name(),
            Field::Real(f) => f.name(),
            Field::Set(e) => e.name(),
            Field::SmallInt(i) => i.name(),
            Field::Text(t) => t.name(),
            Field::Time(t) => t.name(),
            Field::Timestamp(d) => d.name(),
            Field::TinyBlob(s) => s.name(),
            Field::TinyInt(i) => i.name(),
            Field::TinyText(t) => t.name(),
            Field::VarBinary(b) => b.name(),
            Field::VarChar(c) => c.name(),
            Field::Year(d) => d.name(),
        }
    }
    pub fn kind(&self) -> FieldKind {
        match self {
            Field::BigInt(_) => FieldKind::BigInt,
            Field::Binary(_) => FieldKind::Binary,
            Field::Bit(_) => FieldKind::Bit,
            Field::Blob(_) => FieldKind::Blob,
            Field::Char(_) => FieldKind::Char,
            Field::Date(_) => FieldKind::Date,
            Field::DateTime(_) => FieldKind::DateTime,
            Field::Decimal(_) => FieldKind::Decimal,
            Field::Double(_) => FieldKind::Double,
            Field::Enum(_) => FieldKind::Enum,
            Field::Float(_) => FieldKind::Float,
            Field::Geometry(_) => FieldKind::Geometry,
            Field::GeometryCollection(_) => FieldKind::GeometryCollection,
            Field::Int(_) => FieldKind::Int,
            Field::Integer(_) => FieldKind::Integer,
            Field::Json(_) => FieldKind::Json,
            Field::LineString(_) => FieldKind::LineString,
            Field::LongBlob(_) => FieldKind::LongBlob,
            Field::LongText(_) => FieldKind::LongText,
            Field::MediumBlob(_) => FieldKind::MediumBlob,
            Field::MediumInt(_) => FieldKind::MediumInt,
            Field::MediumText(_) => FieldKind::MediumText,
            Field::MultiLineString(_) => FieldKind::MultiLineString,
            Field::MultiPoint(_) => FieldKind::MultiPoint,
            Field::MultiPolygon(_) => FieldKind::MultiPolygon,
            Field::Numeric(_) => FieldKind::Numeric,
            Field::Point(_) => FieldKind::Point,
            Field::Polygon(_) => FieldKind::Polygon,
            Field::Real(_) => FieldKind::Real,
            Field::Set(_) => FieldKind::Set,
            Field::SmallInt(_) => FieldKind::SmallInt,
            Field::Text(_) => FieldKind::Text,
            Field::Time(_) => FieldKind::Time,
            Field::Timestamp(_) => FieldKind::Timestamp,
            Field::TinyBlob(_) => FieldKind::TinyBlob,
            Field::TinyInt(_) => FieldKind::TinyInt,
            Field::TinyText(_) => FieldKind::TinyText,
            Field::VarBinary(_) => FieldKind::VarBinary,
            Field::VarChar(_) => FieldKind::VarChar,
            Field::Year(_) => FieldKind::Year,
        }
    }
    pub fn kind_str(&self) -> &str {
        match self {
            Field::BigInt(_) => "bigint",
            Field::Binary(_) => "binary",
            Field::Bit(_) => "bit",
            Field::Blob(_) => "blob",
            Field::Char(_) => "char",
            Field::Date(_) => "date",
            Field::DateTime(_) => "datetime",
            Field::Decimal(_) => "decimal",
            Field::Double(_) => "double",
            Field::Enum(_) => "enum",
            Field::Float(_) => "float",
            Field::Geometry(_) => "geometry",
            Field::GeometryCollection(_) => "geometrycollection",
            Field::Int(_) => "int",
            Field::Integer(_) => "integer",
            Field::Json(_) => "json",
            Field::LineString(_) => "linestring",
            Field::LongBlob(_) => "longblob",
            Field::LongText(_) => "longtext",
            Field::MediumBlob(_) => "mediumblob",
            Field::MediumInt(_) => "mediumint",
            Field::MediumText(_) => "mediumtext",
            Field::MultiLineString(_) => "multilinestring",
            Field::MultiPoint(_) => "multipoint",
            Field::MultiPolygon(_) => "multipolygon",
            Field::Numeric(_) => "numeric",
            Field::Point(_) => "point",
            Field::Polygon(_) => "polygon",
            Field::Real(_) => "real",
            Field::Set(_) => "set",
            Field::SmallInt(_) => "smallint",
            Field::Text(_) => "text",
            Field::Time(_) => "time",
            Field::Timestamp(_) => "timestamp",
            Field::TinyBlob(_) => "tinyblob",
            Field::TinyInt(_) => "tinyint",
            Field::TinyText(_) => "tinytext",
            Field::VarBinary(_) => "varbinary",
            Field::VarChar(_) => "varchar",
            Field::Year(_) => "year",
        }
    }
    pub fn key(&self) -> bool {
        match self {
            Field::BigInt(i) => i.key(),
            Field::Binary(b) => b.key(),
            Field::Bit(b) => b.key(),
            Field::Blob(s) => s.key(),
            Field::Char(c) => c.key(),
            Field::Date(d) => d.key(),
            Field::DateTime(dt) => dt.key(),
            Field::Decimal(d) => d.key(),
            Field::Double(f) => f.key(),
            Field::Enum(e) => e.key(),
            Field::Float(f) => f.key(),
            Field::Geometry(g) => g.key(),
            Field::GeometryCollection(g) => g.key(),
            Field::Int(i) => i.key(),
            Field::Integer(i) => i.key(),
            Field::Json(s) => s.key(),
            Field::LineString(s) => s.key(),
            Field::LongBlob(s) => s.key(),
            Field::LongText(lt) => lt.key(),
            Field::MediumBlob(m) => m.key(),
            Field::MediumInt(i) => i.key(),
            Field::MediumText(t) => t.key(),
            Field::MultiLineString(s) => s.key(),
            Field::MultiPoint(s) => s.key(),
            Field::MultiPolygon(s) => s.key(),
            Field::Numeric(d) => d.key(),
            Field::Point(s) => s.key(),
            Field::Polygon(s) => s.key(),
            Field::Real(f) => f.key(),
            Field::Set(e) => e.key(),
            Field::SmallInt(i) => i.key(),
            Field::Text(t) => t.key(),
            Field::Time(t) => t.key(),
            Field::Timestamp(d) => d.key(),
            Field::TinyBlob(s) => s.key(),
            Field::TinyInt(i) => i.key(),
            Field::TinyText(t) => t.key(),
            Field::VarBinary(b) => b.key(),
            Field::VarChar(c) => c.key(),
            Field::Year(d) => d.key(),
        }
    }
    pub fn default_value(&self) -> Option<&str> {
        match self {
            Field::BigInt(i) => i.default_value(),
            Field::Binary(b) => b.default_value(),
            Field::Bit(b) => b.default_value(),
            Field::Blob(s) => s.default_value(),
            Field::Char(c) => c.default_value(),
            Field::Date(d) => d.default_value(),
            Field::DateTime(dt) => dt.default_value(),
            Field::Decimal(d) => d.default_value(),
            Field::Double(f) => f.default_value(),
            Field::Enum(e) => e.default_value(),
            Field::Float(f) => f.default_value(),
            Field::Geometry(g) => g.default_value(),
            Field::GeometryCollection(g) => g.default_value(),
            Field::Int(i) => i.default_value(),
            Field::Integer(i) => i.default_value(),
            Field::Json(s) => s.default_value(),
            Field::LineString(s) => s.default_value(),
            Field::LongBlob(s) => s.default_value(),
            Field::LongText(lt) => lt.default_value(),
            Field::MediumBlob(m) => m.default_value(),
            Field::MediumInt(i) => i.default_value(),
            Field::MediumText(t) => t.default_value(),
            Field::MultiLineString(s) => s.default_value(),
            Field::MultiPoint(s) => s.default_value(),
            Field::MultiPolygon(s) => s.default_value(),
            Field::Numeric(d) => d.default_value(),
            Field::Point(s) => s.default_value(),
            Field::Polygon(s) => s.default_value(),
            Field::Real(f) => f.default_value(),
            Field::Set(e) => e.default_value(),
            Field::SmallInt(i) => i.default_value(),
            Field::Text(t) => t.default_value(),
            Field::Time(t) => t.default_value(),
            Field::Timestamp(d) => d.default_value(),
            Field::TinyBlob(s) => s.default_value(),
            Field::TinyInt(i) => i.default_value(),
            Field::TinyText(t) => t.default_value(),
            Field::VarBinary(b) => b.default_value(),
            Field::VarChar(c) => c.default_value(),
            Field::Year(d) => d.default_value(),
        }
    }

    pub fn not_null(&self) -> bool {
        match self {
            Field::BigInt(i) => i.not_null(),
            Field::Binary(b) => b.not_null(),
            Field::Bit(b) => b.not_null(),
            Field::Blob(s) => s.not_null(),
            Field::Char(c) => c.not_null(),
            Field::Date(d) => d.not_null(),
            Field::DateTime(dt) => dt.not_null(),
            Field::Decimal(d) => d.not_null(),
            Field::Double(f) => f.not_null(),
            Field::Enum(e) => e.not_null(),
            Field::Float(f) => f.not_null(),
            Field::Geometry(g) => g.not_null(),
            Field::GeometryCollection(g) => g.not_null(),
            Field::Int(i) => i.not_null(),
            Field::Integer(i) => i.not_null(),
            Field::Json(s) => s.not_null(),
            Field::LineString(s) => s.not_null(),
            Field::LongBlob(s) => s.not_null(),
            Field::LongText(lt) => lt.not_null(),
            Field::MediumBlob(m) => m.not_null(),
            Field::MediumInt(i) => i.not_null(),
            Field::MediumText(t) => t.not_null(),
            Field::MultiLineString(s) => s.not_null(),
            Field::MultiPoint(s) => s.not_null(),
            Field::MultiPolygon(s) => s.not_null(),
            Field::Numeric(d) => d.not_null(),
            Field::Point(s) => s.not_null(),
            Field::Polygon(s) => s.not_null(),
            Field::Real(f) => f.not_null(),
            Field::Set(e) => e.not_null(),
            Field::SmallInt(i) => i.not_null(),
            Field::Text(t) => t.not_null(),
            Field::Time(t) => t.not_null(),
            Field::Timestamp(d) => d.not_null(),
            Field::TinyBlob(s) => s.not_null(),
            Field::TinyInt(i) => i.not_null(),
            Field::TinyText(t) => t.not_null(),
            Field::VarBinary(b) => b.not_null(),
            Field::VarChar(c) => c.not_null(),
            Field::Year(d) => d.not_null(),
        }
    }
    pub fn extra(&self) -> Option<&str> {
        match self {
            Field::BigInt(i) => i.extra(),
            Field::Binary(b) => b.extra(),
            Field::Bit(b) => b.extra(),
            Field::Blob(s) => s.extra(),
            Field::Char(c) => c.extra(),
            Field::Date(d) => d.extra(),
            Field::DateTime(dt) => dt.extra(),
            Field::Decimal(d) => d.extra(),
            Field::Double(f) => f.extra(),
            Field::Enum(e) => e.extra(),
            Field::Float(f) => f.extra(),
            Field::Geometry(g) => g.extra(),
            Field::GeometryCollection(g) => g.extra(),
            Field::Int(i) => i.extra(),
            Field::Integer(i) => i.extra(),
            Field::Json(s) => s.extra(),
            Field::LineString(s) => s.extra(),
            Field::LongBlob(s) => s.extra(),
            Field::LongText(lt) => lt.extra(),
            Field::MediumBlob(m) => m.extra(),
            Field::MediumInt(i) => i.extra(),
            Field::MediumText(t) => t.extra(),
            Field::MultiLineString(s) => s.extra(),
            Field::MultiPoint(s) => s.extra(),
            Field::MultiPolygon(s) => s.extra(),
            Field::Numeric(d) => d.extra(),
            Field::Point(s) => s.extra(),
            Field::Polygon(s) => s.extra(),
            Field::Real(f) => f.extra(),
            Field::Set(e) => e.extra(),
            Field::SmallInt(i) => i.extra(),
            Field::Text(t) => t.extra(),
            Field::Time(t) => t.extra(),
            Field::Timestamp(d) => d.extra(),
            Field::TinyBlob(s) => s.extra(),
            Field::TinyInt(i) => i.extra(),
            Field::TinyText(t) => t.extra(),
            Field::VarBinary(b) => b.extra(),
            Field::VarChar(c) => c.extra(),
            Field::Year(d) => d.extra(),
        }
    }
    pub fn comment(&self) -> Option<&str> {
        match self {
            Field::BigInt(i) => i.comment(),
            Field::Binary(b) => b.comment(),
            Field::Bit(b) => b.comment(),
            Field::Blob(s) => s.comment(),
            Field::Char(c) => c.comment(),
            Field::Date(d) => d.comment(),
            Field::DateTime(dt) => dt.comment(),
            Field::Decimal(d) => d.comment(),
            Field::Double(f) => f.comment(),
            Field::Enum(e) => e.comment(),
            Field::Float(f) => f.comment(),
            Field::Geometry(g) => g.comment(),
            Field::GeometryCollection(g) => g.comment(),
            Field::Int(i) => i.comment(),
            Field::Integer(i) => i.comment(),
            Field::Json(s) => s.comment(),
            Field::LineString(s) => s.comment(),
            Field::LongBlob(s) => s.comment(),
            Field::LongText(lt) => lt.comment(),
            Field::MediumBlob(m) => m.comment(),
            Field::MediumInt(i) => i.comment(),
            Field::MediumText(t) => t.comment(),
            Field::MultiLineString(s) => s.comment(),
            Field::MultiPoint(s) => s.comment(),
            Field::MultiPolygon(s) => s.comment(),
            Field::Numeric(d) => d.comment(),
            Field::Point(s) => s.comment(),
            Field::Polygon(s) => s.comment(),
            Field::Real(f) => f.comment(),
            Field::Set(e) => e.comment(),
            Field::SmallInt(i) => i.comment(),
            Field::Text(t) => t.comment(),
            Field::Time(t) => t.comment(),
            Field::Timestamp(d) => d.comment(),
            Field::TinyBlob(s) => s.comment(),
            Field::TinyInt(i) => i.comment(),
            Field::TinyText(t) => t.comment(),
            Field::VarBinary(b) => b.comment(),
            Field::VarChar(c) => c.comment(),
            Field::Year(d) => d.comment(),
        }
    }
    pub fn get_create_str(&self) -> String {
        match self {
            Field::BigInt(i) => i.get_create_str(FieldKind::BigInt.to_string()),
            Field::Binary(b) => b.get_create_str(FieldKind::Binary.to_string()),
            Field::Bit(b) => b.get_create_str(FieldKind::Bit.to_string()),
            Field::Blob(s) => s.get_create_str(FieldKind::Blob.to_string()),
            Field::Char(c) => c.get_create_str(FieldKind::Char.to_string()),
            Field::Date(d) => d.get_create_str(FieldKind::Date.to_string()),
            Field::DateTime(dt) => dt.get_create_str(FieldKind::DateTime.to_string()),
            Field::Decimal(d) => d.get_create_str(FieldKind::Decimal.to_string()),
            Field::Double(f) => f.get_create_str(FieldKind::Double.to_string()),
            Field::Enum(e) => e.get_create_str(FieldKind::Enum.to_string()),
            Field::Float(f) => f.get_create_str(FieldKind::Float.to_string()),
            Field::Geometry(g) => g.get_create_str(FieldKind::Geometry.to_string()),
            Field::GeometryCollection(g) => g.get_create_str("geometrycollection".to_string()),
            Field::Int(i) => i.get_create_str(FieldKind::Int.to_string()),
            Field::Integer(i) => i.get_create_str(FieldKind::Integer.to_string()),
            Field::Json(s) => s.get_create_str(FieldKind::Json.to_string()),
            Field::LineString(s) => s.get_create_str(FieldKind::LineString.to_string()),
            Field::LongBlob(s) => s.get_create_str(FieldKind::LongBlob.to_string()),
            Field::LongText(lt) => lt.get_create_str(FieldKind::LongText.to_string()),
            Field::MediumBlob(m) => m.get_create_str(FieldKind::MediumBlob.to_string()),
            Field::MediumInt(i) => i.get_create_str(FieldKind::MediumInt.to_string()),
            Field::MediumText(t) => t.get_create_str(FieldKind::MediumText.to_string()),
            Field::MultiLineString(s) => s.get_create_str(FieldKind::MultiLineString.to_string()),
            Field::MultiPoint(s) => s.get_create_str(FieldKind::MultiPoint.to_string()),
            Field::MultiPolygon(s) => s.get_create_str(FieldKind::MultiPolygon.to_string()),
            Field::Numeric(d) => d.get_create_str(FieldKind::Numeric.to_string()),
            Field::Point(s) => s.get_create_str(FieldKind::Point.to_string()),
            Field::Polygon(s) => s.get_create_str(FieldKind::Polygon.to_string()),
            Field::Real(f) => f.get_create_str(FieldKind::Real.to_string()),
            Field::Set(e) => e.get_create_str(FieldKind::Set.to_string()),
            Field::SmallInt(i) => i.get_create_str(FieldKind::SmallInt.to_string()),
            Field::Text(t) => t.get_create_str(FieldKind::Text.to_string()),
            Field::Time(t) => t.get_create_str(FieldKind::Time.to_string()),
            Field::Timestamp(d) => d.get_create_str(FieldKind::Timestamp.to_string()),
            Field::TinyBlob(s) => s.get_create_str(FieldKind::TinyBlob.to_string()),
            Field::TinyInt(i) => i.get_create_str(FieldKind::TinyInt.to_string()),
            Field::TinyText(t) => t.get_create_str(FieldKind::TinyText.to_string()),
            Field::VarBinary(b) => b.get_create_str(FieldKind::VarBinary.to_string()),
            Field::VarChar(c) => c.get_create_str(FieldKind::VarChar.to_string()),
            Field::Year(d) => d.get_create_str(FieldKind::Year.to_string()),
        }
    }
    pub fn get_change_str(&self, old: &Field) -> Option<String> {
        match self {
            Field::BigInt(i) => {
                if let Field::BigInt(oi) = old {
                    i.get_change_str(FieldKind::BigInt.to_string(), oi)
                } else {
                    None
                }
            }
            Field::Binary(b) => {
                if let Field::Binary(ob) = old {
                    b.get_change_str(FieldKind::Binary.to_string(), ob)
                } else {
                    None
                }
            }
            Field::Bit(b) => {
                if let Field::Bit(ob) = old {
                    b.get_change_str(FieldKind::Bit.to_string(), ob)
                } else {
                    None
                }
            }
            Field::Blob(s) => {
                if let Field::Blob(ob) = old {
                    s.get_change_str(FieldKind::Blob.to_string(), ob)
                } else {
                    None
                }
            }
            Field::Char(c) => {
                if let Field::Char(oc) = old {
                    c.get_change_str(FieldKind::Char.to_string(), oc)
                } else {
                    None
                }
            }
            Field::Date(d) => {
                if let Field::Date(od) = old {
                    d.get_change_str(FieldKind::Date.to_string(), od)
                } else {
                    None
                }
            }
            Field::DateTime(dt) => {
                if let Field::DateTime(od) = old {
                    dt.get_change_str(FieldKind::DateTime.to_string(), od)
                } else {
                    None
                }
            }
            Field::Decimal(d) => {
                if let Field::Decimal(od) = old {
                    d.get_change_str(FieldKind::Decimal.to_string(), od)
                } else {
                    None
                }
            }
            Field::Double(f) => {
                if let Field::Double(od) = old {
                    f.get_change_str(FieldKind::Double.to_string(), od)
                } else {
                    None
                }
            }
            Field::Enum(e) => {
                if let Field::Enum(oe) = old {
                    e.get_change_str(FieldKind::Enum.to_string(), oe)
                } else {
                    None
                }
            }
            Field::Float(f) => {
                if let Field::Float(of) = old {
                    f.get_change_str(FieldKind::Float.to_string(), of)
                } else {
                    None
                }
            }
            Field::Geometry(g) => {
                if let Field::Geometry(og) = old {
                    g.get_change_str(FieldKind::Geometry.to_string(), og)
                } else {
                    None
                }
            }
            Field::GeometryCollection(g) => {
                if let Field::GeometryCollection(og) = old {
                    g.get_change_str(FieldKind::GeometryCollection.to_string(), og)
                } else {
                    None
                }
            }
            Field::Int(i) => {
                if let Field::Int(oi) = old {
                    i.get_change_str(FieldKind::Int.to_string(), oi)
                } else {
                    None
                }
            }
            Field::Integer(i) => {
                if let Field::Integer(oi) = old {
                    i.get_change_str(FieldKind::Integer.to_string(), oi)
                } else {
                    None
                }
            }
            Field::Json(s) => {
                if let Field::Json(oj) = old {
                    s.get_change_str(FieldKind::Json.to_string(), oj)
                } else {
                    None
                }
            }
            Field::LineString(s) => {
                if let Field::LineString(ol) = old {
                    s.get_change_str(FieldKind::LineString.to_string(), ol)
                } else {
                    None
                }
            }
            Field::LongBlob(s) => {
                if let Field::LongBlob(ol) = old {
                    s.get_change_str(FieldKind::LongBlob.to_string(), ol)
                } else {
                    None
                }
            }
            Field::LongText(lt) => {
                if let Field::LongText(ol) = old {
                    lt.get_change_str(FieldKind::LongText.to_string(), ol)
                } else {
                    None
                }
            }
            Field::MediumBlob(m) => {
                if let Field::MediumBlob(om) = old {
                    m.get_change_str(FieldKind::MediumBlob.to_string(), om)
                } else {
                    None
                }
            }
            Field::MediumInt(i) => {
                if let Field::MediumInt(oi) = old {
                    i.get_change_str(FieldKind::MediumInt.to_string(), oi)
                } else {
                    None
                }
            }
            Field::MediumText(t) => {
                if let Field::MediumText(om) = old {
                    t.get_change_str(FieldKind::MediumText.to_string(), om)
                } else {
                    None
                }
            }
            Field::MultiLineString(s) => {
                if let Field::MultiLineString(om) = old {
                    s.get_change_str(FieldKind::MultiLineString.to_string(), om)
                } else {
                    None
                }
            }
            Field::MultiPoint(s) => {
                if let Field::MultiPoint(om) = old {
                    s.get_change_str(FieldKind::MultiPoint.to_string(), om)
                } else {
                    None
                }
            }
            Field::MultiPolygon(s) => {
                if let Field::MultiPolygon(om) = old {
                    s.get_change_str(FieldKind::MultiPolygon.to_string(), om)
                } else {
                    None
                }
            }
            Field::Numeric(d) => {
                if let Field::Numeric(on) = old {
                    d.get_change_str(FieldKind::Numeric.to_string(), on)
                } else {
                    None
                }
            }
            Field::Point(s) => {
                if let Field::Point(op) = old {
                    s.get_change_str(FieldKind::Point.to_string(), op)
                } else {
                    None
                }
            }
            Field::Polygon(s) => {
                if let Field::Polygon(op) = old {
                    s.get_change_str(FieldKind::Polygon.to_string(), op)
                } else {
                    None
                }
            }
            Field::Real(f) => {
                if let Field::Real(or) = old {
                    f.get_change_str(FieldKind::Real.to_string(), or)
                } else {
                    None
                }
            }
            Field::Set(e) => {
                if let Field::Set(os) = old {
                    e.get_change_str(FieldKind::Set.to_string(), os)
                } else {
                    None
                }
            }
            Field::SmallInt(i) => {
                if let Field::SmallInt(os) = old {
                    i.get_change_str(FieldKind::SmallInt.to_string(), os)
                } else {
                    None
                }
            }
            Field::Text(t) => {
                if let Field::Text(ot) = old {
                    t.get_change_str(FieldKind::Text.to_string(), ot)
                } else {
                    None
                }
            }
            Field::Time(t) => {
                if let Field::Time(ot) = old {
                    t.get_change_str(FieldKind::Time.to_string(), ot)
                } else {
                    None
                }
            }
            Field::Timestamp(d) => {
                if let Field::Timestamp(od) = old {
                    d.get_change_str(FieldKind::Timestamp.to_string(), od)
                } else {
                    None
                }
            }
            Field::TinyBlob(s) => {
                if let Field::TinyBlob(ot) = old {
                    s.get_change_str(FieldKind::TinyBlob.to_string(), ot)
                } else {
                    None
                }
            }
            Field::TinyInt(i) => {
                if let Field::TinyInt(ot) = old {
                    i.get_change_str(FieldKind::TinyInt.to_string(), ot)
                } else {
                    None
                }
            }
            Field::TinyText(t) => {
                if let Field::TinyText(ot) = old {
                    t.get_change_str(FieldKind::TinyText.to_string(), ot)
                } else {
                    None
                }
            }
            Field::VarBinary(b) => {
                if let Field::VarBinary(ov) = old {
                    b.get_change_str(FieldKind::VarBinary.to_string(), ov)
                } else {
                    None
                }
            }
            Field::VarChar(c) => {
                if let Field::VarChar(ov) = old {
                    c.get_change_str(FieldKind::VarChar.to_string(), ov)
                } else {
                    None
                }
            }
            Field::Year(d) => {
                if let Field::Year(oy) = old {
                    d.get_change_str(FieldKind::Year.to_string(), oy)
                } else {
                    None
                }
            }
        }
    }
    pub fn get_add_str(&self) -> String {
        format!("ADD {}", self.get_create_str())
    }
    pub fn get_drop_str(&self) -> String {
        format!("DROP COLUMN `{}`", self.name())
    }
}

pub async fn get_mysql_field_names(pool: &MySqlPool, table: &str) -> Result<Vec<String>> {
    let fields: Vec<String> = sqlx::query(format!("show columns from {}", table).as_str())
        .fetch_all(pool)
        .await?
        .iter()
        .map(|t| t.try_get("Field").unwrap())
        .collect();
    Ok(fields)
}

pub fn unsigned(unsigned: bool) -> String {
    format!(
        "{}",
        if unsigned {
            String::from(" UNSIGNED")
        } else {
            String::from("")
        }
    )
}

pub fn zerofill(zerofill: bool) -> String {
    format!(
        "{}",
        if zerofill {
            String::from(" ZEROFILL")
        } else {
            String::from("")
        }
    )
}

pub fn default_value(default_value: Option<&str>, quote: bool) -> String {
    if let Some(d) = default_value {
        if !d.is_empty() {
            if quote {
                format!(" DEFAULT '{}'", d)
            } else {
                format!(" DEFAULT {}", d)
            }
        } else {
            String::from("")
        }
    } else {
        String::from("")
    }
}

pub fn length(length: Option<&str>) -> String {
    let l = length.unwrap();
    if !l.is_empty() {
        format!("({})", l)
    } else {
        String::from("")
    }
}

pub fn length_decimal(length: Option<&str>, decimal: Option<&str>) -> String {
    let l = length.unwrap();
    let d = decimal.unwrap();
    if !l.is_empty() {
        if !d.is_empty() {
            format!("({},{})", l, d)
        } else {
            format!("({})", l)
        }
    } else {
        String::from("")
    }
}

pub fn character_set(character_set: Option<&str>) -> String {
    if let Some(c) = character_set {
        if !c.is_empty() {
            format!(" CHARACTER SET {}", c)
        } else {
            String::from("")
        }
    } else {
        String::from("")
    }
}

pub fn collation(collation: Option<&str>) -> String {
    if let Some(c) = collation {
        if !c.is_empty() {
            format!(" COLLATE {}", c)
        } else {
            String::from("")
        }
    } else {
        String::from("")
    }
}

pub fn not_null(not_null: bool) -> String {
    format!("{}", if not_null { " NOT NULL" } else { " NULL" })
}

pub fn auto_increment(auto_increment: bool) -> String {
    format!(
        "{}",
        if auto_increment {
            " AUTO_INCREMENT"
        } else {
            ""
        }
    )
}

pub fn on_update(on_update: bool, length: Option<&str>) -> String {
    let l = length.unwrap();
    if on_update {
        if !l.is_empty() {
            format!(" ON UPDATE CURRENT_TIMESTAMP ({})", l)
        } else {
            format!(" ON UPDATE CURRENT_TIMESTAMP")
        }
    } else {
        String::from("")
    }
}

pub fn comment(comment: Option<&str>) -> String {
    let c = comment.unwrap();
    if !c.is_empty() {
        format!(" COMMENT '{}'", c)
    } else {
        String::from("")
    }
}

pub fn get_mysql_field_value(field: &Field, row: &MySqlRow) -> Option<String> {
    let col_name = field.name();

    fn get_value<'r, U>(name: &str, row: &'r MySqlRow) -> Option<String>
    where
        U: std::fmt::Display + sqlx::Decode<'r, MySql> + sqlx::Type<MySql>,
    {
        let i: Option<U> = row.try_get(name).unwrap();
        i.map(|i| i.to_string())
    }
    fn get_numeric<'r, I, U>(name: &str, is_unsigned: bool, row: &'r MySqlRow) -> Option<String>
    where
        I: std::fmt::Display + sqlx::Decode<'r, MySql> + sqlx::Type<MySql>,
        U: std::fmt::Display + sqlx::Decode<'r, MySql> + sqlx::Type<MySql>,
    {
        if is_unsigned {
            get_value::<U>(name, row)
        } else {
            get_value::<I>(name, row)
        }
    }

    match field {
        Field::VarChar(_) | Field::Char(_) => get_value::<String>(col_name, row),
        Field::Binary(_)
        | Field::VarBinary(_)
        | Field::Blob(_)
        | Field::TinyBlob(_)
        | Field::MediumBlob(_)
        | Field::LongBlob(_) => {
            let i: Option<Vec<u8>> = row.try_get(col_name).unwrap();
            i.map(|i| String::from_utf8(i).unwrap_or_default())
        }
        Field::TinyInt(field) => get_numeric::<i8, u8>(col_name, field.unsigned, row),
        Field::SmallInt(field) => get_numeric::<i16, u16>(col_name, field.unsigned, row),
        Field::MediumInt(field) => get_numeric::<i32, u32>(col_name, field.unsigned, row),
        Field::Int(field) | Field::Integer(field) => {
            get_numeric::<i32, u32>(col_name, field.unsigned, row)
        }
        Field::BigInt(field) => get_numeric::<i64, u64>(col_name, field.unsigned, row),
        Field::Numeric(_) | Field::Decimal(_) => get_value::<BigDecimal>(col_name, row),
        Field::Float(_) => get_value::<f32>(col_name, row),
        Field::Double(_) => get_value::<f64>(col_name, row),
        Field::Real(_) => get_value::<f64>(col_name, row),
        Field::Enum(_) => get_value::<String>(col_name, row),
        Field::Set(_) => get_value::<String>(col_name, row),
        Field::Json(_) => get_value::<JsonValue>(col_name, row),
        Field::Text(_) | Field::TinyText(_) | Field::MediumText(_) | Field::LongText(_) => {
            get_value::<String>(col_name, row)
        }
        Field::Year(_) => get_value::<u16>(col_name, row),
        Field::Date(_) => {
            let d: Option<NaiveDate> = row.try_get(field.name()).unwrap();
            d.map(|d| d.format("%Y-%m-%d").to_string())
        }
        Field::Time(_) => {
            let d: Option<NaiveTime> = row.try_get(field.name()).unwrap();
            d.map(|d| d.format("%H:%M:%S").to_string())
        }
        Field::DateTime(field) | Field::Timestamp(field) => {
            let d: Option<DateTime<Utc>> = row.try_get(field.name()).unwrap();
            d.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        }
        _ => None,
    }
}

pub fn get_mysql_column_value(column: &MySqlColumn, row: &MySqlRow) -> Option<String> {
    let col_name = column.name();
    fn get_value<'r, T>(col_name: &str, row: &'r MySqlRow) -> Option<String>
    where
        T: std::fmt::Display + sqlx::Decode<'r, MySql> + sqlx::Type<MySql>,
    {
        let i: Option<T> = row.try_get(col_name).unwrap();
        i.map(|i| i.to_string())
    }

    match column.type_info().name() {
        "VARCHAR" | "CHAR" => get_value::<String>(col_name, row),
        "TINYINT" => get_value::<i8>(col_name, row),
        "TINYINT UNSIGNED" => get_value::<u8>(col_name, row),
        "SMALLINT" => get_value::<i16>(col_name, row),
        "SMALLINT UNSIGNED" => get_value::<u16>(col_name, row),
        "MEDIUMINT" => get_value::<i32>(col_name, row),
        "MEDIUMINT UNSIGNED" => get_value::<u32>(col_name, row),
        "INT UNSIGNED" | "INTEGER UNSIGNED" => get_value::<u32>(col_name, row),
        "INT" | "INTEGER" => get_value::<i32>(col_name, row),
        "BIGINT" => get_value::<i64>(col_name, row),
        "BIGINT UNSIGNED" => get_value::<u64>(col_name, row),
        "NUMERIC" | "DECIMAL" => get_value::<BigDecimal>(col_name, row),
        "FLOAT" => get_value::<f32>(col_name, row),
        "DOUBLE" => get_value::<f64>(col_name, row),
        "REAL" => get_value::<f64>(col_name, row),
        "JSON" => get_value::<JsonValue>(col_name, row),
        "BIT" => None,
        "ENUM" => get_value::<String>(col_name, row),
        "SET" => get_value::<String>(col_name, row),
        "TEXT" | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT" => get_value::<String>(col_name, row),
        "YEAR" => get_value::<u16>(col_name, row),
        "DATE" => {
            let d: Option<NaiveDate> = row.try_get(col_name).unwrap();
            d.map(|d| d.format("%Y-%m-%d").to_string())
        }
        "TIME" => {
            let d: Option<NaiveTime> = row.try_get(col_name).unwrap();
            d.map(|d| d.format("%H:%M:%S").to_string())
        }
        "DATETIME" | "TIMESTAMP" => {
            let d: Option<DateTime<Utc>> = row.try_get(col_name).unwrap();
            d.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        }
        _ => None,
    }
}
pub fn convert_show_column_to_mysql_fields(fields: Vec<MySqlRow>) -> Vec<Field> {
    fields
        .iter()
        .map(|r| {
            let kind: String = r.try_get("Type").unwrap();
            let reg = Regex::new(
                r"(?P<kind>bigint|binary|bit|blob|char|datetime|date|decimal|double|enum|float|geometry|geomcollection|int|integer|json|linestring|longblob|longtext|mediumblob|mediumint|mediumtext|multilinestring|multipoint|multipolygon|numeric|point|polygon|real|set|smallint|text|timestamp|time|tinyblob|tinyint|tinytext|varbinary|varchar|year)(\((?P<length>\d+)\))?(\((?P<num>\d+),(?P<decimal>\d+)\))?(\((?P<options>('.+',?)+)\))?(\s(?P<unsigned>unsigned))?(\s(?P<zerofill>zerofill))?",
            )
            .unwrap();
            let caps = reg.captures(kind.as_str()).unwrap();

            let name: String = r.try_get("Field").unwrap();
            let null: String = r.try_get("Null").unwrap();
            let not_null = null == "NO";
            let key: String = r.try_get("Key").unwrap();
            let key = key == "PRI";
            let default: Option<String> = r.try_get("Default").unwrap();
            let extra: String = r.try_get("Extra").unwrap();
            let collation: Option<String> = r.try_get("Collation").unwrap();
            let comment: Option<String> = r.try_get("Comment").unwrap();

            match FieldKind::try_from(caps.name("kind").unwrap().as_str()).unwrap() {
                FieldKind::BigInt => Field::BigInt(IntField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").map(|c| c.as_str()),
                    default.as_deref(),
                    caps.name("unsigned").is_some(),
                    caps.name("zerofill").is_some(),
                    comment.as_deref(),
                    &extra,
                )),
                FieldKind::Binary => Field::Binary(BinaryField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").unwrap().as_str(),
                    comment.as_deref(),
                    default.as_deref(),
                )),
                FieldKind::Bit => Field::Bit(BinaryField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").unwrap().as_str(),
                    comment.as_deref(),
                    default.as_deref(),
                )),
                FieldKind::Blob => Field::Blob(SimpleField::new(
                    name.as_str(),
                    not_null,
                    key,
                    comment.as_deref(),
                )),
                FieldKind::Char => Field::Char(CharField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").unwrap().as_str(),
                    comment.as_deref(),
                    default.as_deref(),
                    collation.as_deref().unwrap(),
                )),
                FieldKind::Date => Field::Date(DateField::new(
                    name.as_str(),
                    not_null,
                    key,
                    comment.as_deref(),
                    default.as_deref(),
                )),
                FieldKind::DateTime => Field::DateTime(DateTimeField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").map(|c| c.as_str()),
                    comment.as_deref(),
                    default.as_deref(),
                    &extra,
                )),
                FieldKind::Decimal => Field::Decimal(DecimalField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("num").unwrap().as_str(),
                    caps.name("decimal").unwrap().as_str(),
                    caps.name("unsigned").is_some(),
                    caps.name("zerofill").is_some(),
                    comment.as_deref(),
                    default.as_deref(),
                )),
                FieldKind::Double => Field::Double(FloatField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("num").map(|n| n.as_str()),
                    caps.name("decimal").map(|d| d.as_str()),
                    comment.as_deref(),
                    default.as_deref(),
                    caps.name("unsigned").is_some(),
                    caps.name("zerofill").is_some(),
                    extra.as_str(),
                )),
                FieldKind::Enum => Field::Enum(EnumField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("options").unwrap().as_str(),
                    collation.as_deref().unwrap(),
                    comment.as_deref(),
                    default.as_deref(),
                )),
                FieldKind::Float => Field::Float(FloatField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("num").map(|n| n.as_str()),
                    caps.name("decimal").map(|d| d.as_str()),
                    comment.as_deref(),
                    default.as_deref(),
                    caps.name("unsigned").is_some(),
                    caps.name("zerofill").is_some(),
                    extra.as_str(),
                )),
                FieldKind::Geometry => {
                    Field::Geometry(SimpleField::new(name.as_str(), not_null, key, comment.as_deref()))
                }
                FieldKind::GeometryCollection => {
                    Field::GeometryCollection(SimpleField::new(name.as_str(), not_null, key, comment.as_deref()))
                }
                FieldKind::Int => Field::Int(IntField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").map(|c| c.as_str()),
                    default.as_deref(),
                    caps.name("unsigned").is_some(),
                    caps.name("zerofill").is_some(),
                    comment.as_deref(),
                    extra.as_str(),
                )),
                FieldKind::Integer => Field::Integer(IntField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").map(|c| c.as_str()),
                    default.as_deref(),
                    caps.name("unsigned").is_some(),
                    caps.name("zerofill").is_some(),
                    comment.as_deref(),
                    extra.as_str(),
                )),
                FieldKind::Json => {
                    Field::Json(SimpleField::new(name.as_str(), not_null, key, comment.as_deref()))
                }
                FieldKind::LineString => {
                    Field::LineString(SimpleField::new(name.as_str(), not_null, key, comment.as_deref()))
                }
                FieldKind::LongBlob => {
                    Field::LongBlob(SimpleField::new(name.as_str(), not_null, key, comment.as_deref()))
                }
                FieldKind::LongText => Field::LongText(TextField::new(
                    name.as_str(),
                    not_null,
                    key,
                    collation.as_deref().unwrap(),
                    comment.as_deref(),
                )),
                FieldKind::MediumBlob => {
                    Field::MediumBlob(SimpleField::new(name.as_str(), not_null, key, comment.as_deref()))
                }
                FieldKind::MediumInt => Field::MediumInt(IntField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").map(|c| c.as_str()),
                    default.as_deref(),
                    caps.name("unsigned").is_some(),
                    caps.name("zerofill").is_some(),
                    comment.as_deref(),
                    extra.as_str(),
                )),
                FieldKind::MediumText => Field::MediumText(TextField::new(
                    name.as_str(),
                    not_null,
                    key,
                    collation.as_deref().unwrap(),
                    comment.as_deref(),
                )),
                FieldKind::MultiLineString => {
                    Field::MultiLineString(SimpleField::new(name.as_str(), not_null, key, comment.as_deref()))
                }
                FieldKind::MultiPoint => {
                    Field::MultiPoint(SimpleField::new(name.as_str(), not_null, key, comment.as_deref()))
                }
                FieldKind::MultiPolygon => {
                    Field::MultiPolygon(SimpleField::new(name.as_str(), not_null, key, comment.as_deref()))
                }
                FieldKind::Numeric => Field::Numeric(DecimalField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("num").unwrap().as_str(),
                    caps.name("decimal").unwrap().as_str(),
                    caps.name("unsigned").is_some(),
                    caps.name("zerofill").is_some(),
                    comment.as_deref(),
                    default.as_deref(),
                )),
                FieldKind::Point => {
                    Field::Point(SimpleField::new(name.as_str(), not_null, key, comment.as_deref()))
                }
                FieldKind::Polygon => {
                    Field::Polygon(SimpleField::new(name.as_str(), not_null, key, comment.as_deref()))
                }
                FieldKind::Real => Field::Float(FloatField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("num").map(|n| n.as_str()),
                    caps.name("decimal").map(|d| d.as_str()),
                    comment.as_deref(),
                    default.as_deref(),
                    caps.name("unsigned").is_some(),
                    caps.name("zerofill").is_some(),
                    extra.as_str(),
                )),
                FieldKind::Set => Field::Set(EnumField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("options").unwrap().as_str(),
                    collation.as_deref().unwrap(),
                    comment.as_deref(),
                    default.as_deref(),
                )),
                FieldKind::SmallInt => Field::SmallInt(IntField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").map(|c| c.as_str()),
                    default.as_deref(),
                    caps.name("unsigned").is_some(),
                    caps.name("zerofill").is_some(),
                    comment.as_deref(),
                    extra.as_str(),
                )),
                FieldKind::Text => Field::Text(TextField::new(
                    name.as_str(),
                    not_null,
                    key,
                    collation.as_deref().unwrap(),
                    comment.as_deref(),
                )),
                FieldKind::Time => Field::Time(TimeField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").map(|c| c.as_str()),
                    comment.as_deref(),
                    default.as_deref(),
                )),
                FieldKind::Timestamp => Field::Timestamp(DateTimeField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").map(|c| c.as_str()),
                    comment.as_deref(),
                    default.as_deref(),
                    &extra,
                )),
                FieldKind::TinyBlob => {
                    Field::TinyBlob(SimpleField::new(name.as_str(), not_null, key, comment.as_deref()))
                }
                FieldKind::TinyInt => Field::TinyInt(IntField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").map(|c| c.as_str()),
                    default.as_deref(),
                    caps.name("unsigned").is_some(),
                    caps.name("zerofill").is_some(),
                    comment.as_deref(),
                    extra.as_str(),
                )),
                FieldKind::TinyText => Field::TinyText(TextField::new(
                    name.as_str(),
                    not_null,
                    key,
                    collation.as_deref().unwrap(),
                    comment.as_deref(),
                )),
                FieldKind::VarBinary => Field::VarBinary(BinaryField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").unwrap().as_str(),
                    comment.as_deref(),
                    default.as_deref(),
                )),
                FieldKind::VarChar => Field::VarChar(CharField::new(
                    name.as_str(),
                    not_null,
                    key,
                    caps.name("length").unwrap().as_str(),
                    comment.as_deref(),
                    default.as_deref(),
                    collation.as_deref().unwrap(),
                )),
                FieldKind::Year => Field::Year(DateField::new(
                    name.as_str(),
                    not_null,
                    key,
                    comment.as_deref(),
                    default.as_deref(),
                )),
            }
        })
        .collect::<Vec<Field>>()
}
