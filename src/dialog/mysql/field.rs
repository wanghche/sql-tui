use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::mysql::{
        BinaryField, CharField, Connections, DateField, DateTimeField, DecimalField, EnumField,
        Field, FieldKind, FloatField, IntField, SimpleField, TextField, TimeField,
    },
    pool::{fetch_mysql_query, MySQLPools},
    widget::{Form, FormItem},
};
use anyhow::Result;
use sqlx::Row;
use std::{cell::RefCell, cmp::min, collections::HashMap, rc::Rc};
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};
use uuid::Uuid;

pub struct FieldDialog<'a> {
    id: Option<Uuid>,
    kind: FieldKind,
    form: Form<'a>,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<MySQLPools>>,
    conn_id: Uuid,
}

impl<'a> FieldDialog<'a> {
    pub async fn new(
        kind: FieldKind,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<MySQLPools>>,
        conn_id: &Uuid,
    ) -> Result<FieldDialog<'a>> {
        let mut form = Form::default();
        form.set_title(format!("{} field", kind.to_string()));
        let items = match kind {
            FieldKind::BigInt
            | FieldKind::Int
            | FieldKind::Integer
            | FieldKind::SmallInt
            | FieldKind::MediumInt
            | FieldKind::TinyInt => Self::create_int_form(None),
            FieldKind::Binary | FieldKind::VarBinary | FieldKind::Bit => {
                Self::create_binary_form(None)
            }
            FieldKind::Date | FieldKind::Year => Self::create_date_form(None),
            FieldKind::VarChar | FieldKind::Char => {
                Self::create_char_form(None, conns.clone(), pools.clone(), conn_id).await?
            }
            FieldKind::DateTime | FieldKind::Timestamp => Self::create_datetime_form(None),
            FieldKind::Decimal | FieldKind::Numeric => Self::create_decimal_form(None),
            FieldKind::Double | FieldKind::Float | FieldKind::Real => Self::create_float_form(None),
            FieldKind::LongText | FieldKind::MediumText | FieldKind::Text | FieldKind::TinyText => {
                Self::create_text_form(None, conns.clone(), pools.clone(), conn_id).await?
            }
            FieldKind::Enum | FieldKind::Set => {
                Self::create_enum_form(None, conns.clone(), pools.clone(), conn_id).await?
            }
            FieldKind::Time => Self::create_time_form(None),
            FieldKind::Blob
            | FieldKind::Geometry
            | FieldKind::GeometryCollection
            | FieldKind::Json
            | FieldKind::LineString
            | FieldKind::LongBlob
            | FieldKind::MediumBlob
            | FieldKind::MultiLineString
            | FieldKind::MultiPoint
            | FieldKind::MultiPolygon
            | FieldKind::Point
            | FieldKind::Polygon
            | FieldKind::TinyBlob => Self::create_simple_form(None),
        };

        form.set_items(items);
        Ok(FieldDialog {
            id: None,
            kind,
            form,
            conns,
            pools,
            conn_id: *conn_id,
        })
    }
    pub async fn from_field(
        field: &Field,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<MySQLPools>>,
        conn_id: &Uuid,
    ) -> Result<FieldDialog<'a>> {
        let mut form = Form::default();
        form.set_title(format!("{} field", field.kind()));
        let items = match field {
            Field::BigInt(i)
            | Field::Int(i)
            | Field::Integer(i)
            | Field::SmallInt(i)
            | Field::MediumInt(i)
            | Field::TinyInt(i) => Self::create_int_form(Some(i)),
            Field::Binary(b) | Field::VarBinary(b) | Field::Bit(b) => {
                Self::create_binary_form(Some(b))
            }
            Field::Date(d) | Field::Year(d) => Self::create_date_form(Some(d)),
            Field::VarChar(c) | Field::Char(c) => {
                Self::create_char_form(Some(c), conns.clone(), pools.clone(), conn_id).await?
            }
            Field::DateTime(d) | Field::Timestamp(d) => Self::create_datetime_form(Some(d)),
            Field::Decimal(d) | Field::Numeric(d) => Self::create_decimal_form(Some(d)),
            Field::Double(d) | Field::Float(d) | Field::Real(d) => Self::create_float_form(Some(d)),
            Field::LongText(t) | Field::MediumText(t) | Field::Text(t) | Field::TinyText(t) => {
                Self::create_text_form(Some(t), conns.clone(), pools.clone(), conn_id).await?
            }
            Field::Enum(d) | Field::Set(d) => {
                Self::create_enum_form(Some(d), conns.clone(), pools.clone(), conn_id).await?
            }
            Field::Time(t) => Self::create_time_form(Some(t)),
            Field::Blob(s)
            | Field::Geometry(s)
            | Field::GeometryCollection(s)
            | Field::Json(s)
            | Field::LineString(s)
            | Field::LongBlob(s)
            | Field::MediumBlob(s)
            | Field::MultiLineString(s)
            | Field::MultiPoint(s)
            | Field::MultiPolygon(s)
            | Field::Point(s)
            | Field::Polygon(s)
            | Field::TinyBlob(s) => Self::create_simple_form(Some(s)),
        };
        form.set_items(items);
        Ok(FieldDialog {
            id: Some(field.id().to_owned()),
            kind: field.kind(),
            form,
            conns,
            pools,
            conn_id: *conn_id,
        })
    }
    pub fn get_id(&self) -> Option<&Uuid> {
        self.id.as_ref()
    }
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
    pub fn get_kind(&self) -> &FieldKind {
        &self.kind
    }
    pub fn draw<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        let bounds = f.size();
        let width = min(bounds.width - 2, 60);

        let height = min(self.form.height(), bounds.height);
        let left = (bounds.width - width) / 2;
        let top = (bounds.height - height) / 2;
        let rect = Rect::new(left, top, width, height);
        f.render_widget(Clear, rect);

        self.form.draw(f, rect);
    }

    pub async fn handle_event(
        &mut self,
        key: &Key,
    ) -> Result<DialogResult<HashMap<String, Option<String>>>> {
        let r = self.form.handle_event(key)?;
        match r {
            DialogResult::Changed(name, value) => {
                match name.as_str() {
                    "character set" => {
                        let rows = fetch_mysql_query(
                            self.conns.clone(),
                            self.pools.clone(),
                            &self.conn_id,
                            None,
                            &format!("SHOW COLLATION WHERE Charset='{}'", value),
                        )
                        .await?;
                        self.form.set_item(
                            "collation",
                            FormItem::new_select(
                                "collation".to_string(),
                                rows.iter()
                                    .map(|row| row.try_get("Collation").unwrap())
                                    .collect(),
                                None,
                                true,
                                false,
                            ),
                        );
                    }
                    "key" => {
                        if value == "true" {
                            self.form.set_value("not null", "true");
                        }
                    }
                    "not null" => {
                        if value == "false" {
                            self.form.set_value("key", "false");
                        }
                    }
                    _ => (),
                }
                Ok(DialogResult::Done)
            }
            DialogResult::Confirm(mut map) => {
                if let Some(id) = self.id.as_ref() {
                    map.insert("id".to_string(), Some(id.to_string()));
                }
                Ok(DialogResult::Confirm(map))
            }

            _ => Ok(r),
        }
    }
    fn create_int_form(field: Option<&IntField>) -> Vec<FormItem<'a>> {
        if let Some(f) = field {
            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_check("not null".to_string(), f.not_null(), false),
                FormItem::new_check("key".to_string(), f.key(), false),
                FormItem::new_input("comment".to_string(), f.comment(), true, false, false),
                FormItem::new_input("length".to_string(), f.length(), true, false, false),
                FormItem::new_input(
                    "default value".to_string(),
                    f.default_value(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_check("auto increment".to_string(), f.auto_increment(), false),
                FormItem::new_check("unsigned".to_string(), f.unsigned(), false),
                FormItem::new_check("zerofill".to_string(), f.zerofill(), false),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_check("not null".to_string(), false, false),
                FormItem::new_check("key".to_string(), false, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
                FormItem::new_input("length".to_string(), None, true, false, false),
                FormItem::new_input("default value".to_string(), None, true, false, false),
                FormItem::new_check("auto increment".to_string(), false, false),
                FormItem::new_check("unsigned".to_string(), false, false),
                FormItem::new_check("zerofill".to_string(), false, false),
            ]
        }
    }
    fn create_binary_form(field: Option<&BinaryField>) -> Vec<FormItem<'a>> {
        if let Some(f) = field {
            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_check("not null".to_string(), f.not_null(), false),
                FormItem::new_check("key".to_string(), f.key(), false),
                FormItem::new_input("comment".to_string(), f.comment(), true, false, false),
                FormItem::new_input("length".to_string(), Some(f.length()), true, false, false),
                FormItem::new_input(
                    "default value".to_string(),
                    f.default_value(),
                    true,
                    false,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_check("not null".to_string(), false, false),
                FormItem::new_check("key".to_string(), false, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
                FormItem::new_input("length".to_string(), None, true, false, false),
                FormItem::new_input("default value".to_string(), None, true, false, false),
            ]
        }
    }

    async fn create_char_form(
        field: Option<&CharField>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<MySQLPools>>,
        conn_id: &Uuid,
    ) -> Result<Vec<FormItem<'a>>> {
        let rows = fetch_mysql_query(
            conns.clone(),
            pools.clone(),
            conn_id,
            None,
            "SHOW CHARACTER SET",
        )
        .await?;
        let charsets: Vec<String> = rows
            .iter()
            .map(|row| row.try_get("Charset").unwrap())
            .collect();

        let items = if let Some(f) = field {
            let collations = if let Some(charset) = f.character_set() {
                fetch_mysql_query(
                    conns.clone(),
                    pools.clone(),
                    conn_id,
                    None,
                    &format!("SHOW COLLATION WHERE Charset='{}'", charset),
                )
                .await?
                .iter()
                .map(|row| row.try_get("Collation").unwrap())
                .collect::<Vec<String>>()
            } else {
                vec![]
            };

            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_check("not null".to_string(), f.not_null(), false),
                FormItem::new_check("key".to_string(), f.key(), false),
                FormItem::new_input("comment".to_string(), f.comment(), true, false, false),
                FormItem::new_input("length".to_string(), f.length(), false, false, false),
                FormItem::new_input(
                    "default value".to_string(),
                    f.default_value(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_select(
                    "character set".to_string(),
                    charsets,
                    f.character_set().map(|s| s.to_string()),
                    true,
                    false,
                ),
                FormItem::new_select(
                    "collation".to_string(),
                    collations,
                    f.collation().map(|s| s.to_string()),
                    true,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_check("not null".to_string(), false, false),
                FormItem::new_check("key".to_string(), false, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
                FormItem::new_input("length".to_string(), None, false, false, false),
                FormItem::new_input("default value".to_string(), None, true, false, false),
                FormItem::new_select("character set".to_string(), charsets, None, true, false),
                FormItem::new_select("collation".to_string(), vec![], None, true, false),
            ]
        };
        Ok(items)
    }
    fn create_date_form(field: Option<&DateField>) -> Vec<FormItem<'a>> {
        if let Some(f) = field {
            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_check("not null".to_string(), f.not_null(), false),
                FormItem::new_check("key".to_string(), f.key(), false),
                FormItem::new_input("comment".to_string(), f.comment(), true, false, false),
                FormItem::new_input(
                    "default value".to_string(),
                    f.default_value(),
                    true,
                    false,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_check("not null".to_string(), false, false),
                FormItem::new_check("key".to_string(), false, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
                FormItem::new_input("default value".to_string(), None, true, false, false),
            ]
        }
    }
    fn create_datetime_form(field: Option<&DateTimeField>) -> Vec<FormItem<'a>> {
        if let Some(f) = field {
            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_check("not null".to_string(), f.not_null(), false),
                FormItem::new_check("key".to_string(), f.key(), false),
                FormItem::new_input("comment".to_string(), f.comment(), true, false, false),
                FormItem::new_input("length".to_string(), f.length(), true, false, false),
                FormItem::new_input(
                    "default value".to_string(),
                    f.default_value(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_check("on update".to_string(), f.on_update(), false),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_check("not null".to_string(), false, false),
                FormItem::new_check("key".to_string(), false, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
                FormItem::new_input("length".to_string(), None, true, false, false),
                FormItem::new_input("default value".to_string(), None, true, false, false),
                FormItem::new_check("on update".to_string(), false, false),
            ]
        }
    }
    fn create_decimal_form(field: Option<&DecimalField>) -> Vec<FormItem<'a>> {
        if let Some(f) = field {
            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_check("not null".to_string(), f.not_null(), false),
                FormItem::new_check("key".to_string(), f.key(), false),
                FormItem::new_input("comment".to_string(), f.comment(), true, false, false),
                FormItem::new_input("length".to_string(), f.length(), true, false, false),
                FormItem::new_input("decimal".to_string(), f.decimal(), true, false, false),
                FormItem::new_input(
                    "default value".to_string(),
                    f.default_value(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_check("unsigned".to_string(), f.unsigned(), false),
                FormItem::new_check("zerofill".to_string(), f.zerofill(), false),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_check("not null".to_string(), false, false),
                FormItem::new_check("key".to_string(), false, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
                FormItem::new_input("length".to_string(), None, true, false, false),
                FormItem::new_input("decimal".to_string(), None, true, false, false),
                FormItem::new_input("default value".to_string(), None, true, false, false),
                FormItem::new_check("unsigned".to_string(), false, false),
                FormItem::new_check("zerofill".to_string(), false, false),
            ]
        }
    }
    fn create_float_form(field: Option<&FloatField>) -> Vec<FormItem<'a>> {
        if let Some(f) = field {
            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_check("not null".to_string(), f.not_null(), false),
                FormItem::new_check("key".to_string(), f.key(), false),
                FormItem::new_input("comment".to_string(), f.comment(), true, false, false),
                FormItem::new_input("length".to_string(), f.length(), true, false, false),
                FormItem::new_input("decimal".to_string(), f.decimal(), true, false, false),
                FormItem::new_input(
                    "default value".to_string(),
                    f.default_value(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_check("auto increment".to_string(), f.auto_increment(), false),
                FormItem::new_check("unsigned".to_string(), f.unsigned(), false),
                FormItem::new_check("zerofill".to_string(), f.zerofill(), false),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_check("not null".to_string(), false, false),
                FormItem::new_check("key".to_string(), false, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
                FormItem::new_input("length".to_string(), None, true, false, false),
                FormItem::new_input("decimal".to_string(), None, true, false, false),
                FormItem::new_input("default value".to_string(), None, true, false, false),
                FormItem::new_check("auto increment".to_string(), false, false),
                FormItem::new_check("unsigned".to_string(), false, false),
                FormItem::new_check("zerofill".to_string(), false, false),
            ]
        }
    }
    async fn create_enum_form(
        field: Option<&EnumField>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<MySQLPools>>,
        conn_id: &Uuid,
    ) -> Result<Vec<FormItem<'a>>> {
        let rows = fetch_mysql_query(
            conns.clone(),
            pools.clone(),
            conn_id,
            None,
            "SHOW CHARACTER SET",
        )
        .await?;
        let charsets: Vec<String> = rows
            .iter()
            .map(|row| row.try_get("Charset").unwrap())
            .collect();

        let items = if let Some(f) = field {
            let collations = if let Some(charset) = f.character_set() {
                fetch_mysql_query(
                    conns.clone(),
                    pools.clone(),
                    conn_id,
                    None,
                    &format!("SHOW COLLATION WHERE Charset='{}'", charset),
                )
                .await?
                .iter()
                .map(|row| row.try_get("Collation").unwrap())
                .collect::<Vec<String>>()
            } else {
                vec![]
            };

            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_check("not null".to_string(), f.not_null(), false),
                FormItem::new_check("key".to_string(), f.key(), false),
                FormItem::new_input("comment".to_string(), f.comment(), true, false, false),
                FormItem::new_list("options".to_string(), f.options().to_vec(), false, false),
                FormItem::new_input(
                    "default value".to_string(),
                    f.default_value(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_select(
                    "character set".to_string(),
                    charsets,
                    f.character_set().map(|s| s.to_string()),
                    true,
                    false,
                ),
                FormItem::new_select(
                    "collation".to_string(),
                    collations,
                    f.collation().map(|s| s.to_string()),
                    true,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_check("not null".to_string(), false, false),
                FormItem::new_check("key".to_string(), false, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
                FormItem::new_list("options".to_string(), vec![], false, false),
                FormItem::new_input("default value".to_string(), None, true, false, false),
                FormItem::new_select("character set".to_string(), charsets, None, true, false),
                FormItem::new_select("collation".to_string(), vec![], None, true, false),
            ]
        };
        Ok(items)
    }
    async fn create_text_form(
        field: Option<&TextField>,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<MySQLPools>>,
        conn_id: &Uuid,
    ) -> Result<Vec<FormItem<'a>>> {
        let rows = fetch_mysql_query(
            conns.clone(),
            pools.clone(),
            conn_id,
            None,
            "SHOW CHARACTER SET",
        )
        .await?;
        let charsets: Vec<String> = rows
            .iter()
            .map(|row| row.try_get("Charset").unwrap())
            .collect();

        let items = if let Some(f) = field {
            let collations = if let Some(charset) = f.character_set() {
                fetch_mysql_query(
                    conns.clone(),
                    pools.clone(),
                    conn_id,
                    None,
                    &format!("SHOW COLLATION WHERE Charset='{}'", charset),
                )
                .await?
                .iter()
                .map(|row| row.try_get("Collation").unwrap())
                .collect::<Vec<String>>()
            } else {
                vec![]
            };

            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_check("not null".to_string(), f.not_null(), false),
                FormItem::new_check("key".to_string(), f.key(), false),
                FormItem::new_input("comment".to_string(), f.comment(), true, false, false),
                FormItem::new_select(
                    "character set".to_string(),
                    charsets,
                    f.character_set().map(|s| s.to_string()),
                    true,
                    false,
                ),
                FormItem::new_select(
                    "collation".to_string(),
                    collations,
                    f.collation().map(|s| s.to_string()),
                    true,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_check("not null".to_string(), false, false),
                FormItem::new_check("key".to_string(), false, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
                FormItem::new_select("character set".to_string(), charsets, None, true, false),
                FormItem::new_select("collation".to_string(), vec![], None, true, false),
            ]
        };
        Ok(items)
    }
    fn create_time_form(field: Option<&TimeField>) -> Vec<FormItem<'a>> {
        if let Some(f) = field {
            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_check("not null".to_string(), f.not_null(), false),
                FormItem::new_check("key".to_string(), f.key(), false),
                FormItem::new_input("comment".to_string(), f.comment(), true, false, false),
                FormItem::new_input("length".to_string(), f.length(), true, false, false),
                FormItem::new_input(
                    "default value".to_string(),
                    f.default_value(),
                    true,
                    false,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_check("not null".to_string(), false, false),
                FormItem::new_check("key".to_string(), false, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
                FormItem::new_input("length".to_string(), None, true, false, false),
                FormItem::new_input("default value".to_string(), None, true, false, false),
            ]
        }
    }
    fn create_simple_form(field: Option<&SimpleField>) -> Vec<FormItem<'a>> {
        if let Some(f) = field {
            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_check("not null".to_string(), f.not_null(), false),
                FormItem::new_check("key".to_string(), f.key(), false),
                FormItem::new_input("comment".to_string(), f.comment(), true, false, false),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_check("not null".to_string(), false, false),
                FormItem::new_check("key".to_string(), false, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
            ]
        }
    }
}
