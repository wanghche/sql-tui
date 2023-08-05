use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::{
        mysql::{get_mysql_field_value, Field as MySQLField},
        pg::{get_pg_field_value, Field as PGField, FieldKind as PGFieldKind},
    },
    widget::{Form, FormItem},
};
use anyhow::{Error, Result};
use sqlx::{mysql::MySqlRow, postgres::PgRow};
use std::cmp::min;
use std::collections::HashMap;
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};

pub struct DataDialog<'a> {
    form: Form<'a>,
    old_form: Form<'a>,
}

impl<'a> DataDialog<'a> {
    pub fn new(title: String) -> Self {
        let mut form = Form::default();
        form.set_title(title);
        DataDialog {
            form: form.clone(),
            old_form: form,
        }
    }
    pub fn set_mysql_fields_and_row(&mut self, fields: &[MySQLField], row: Option<&MySqlRow>) {
        let items: Vec<FormItem<'a>> = fields
            .iter()
            .map(|field| {
                let value = if let Some(row) = row {
                    get_mysql_field_value(field, row)
                } else {
                    None
                };
                match field {
                    MySQLField::Enum(f) => FormItem::new_select(
                        field.name().to_string(),
                        f.options.clone(),
                        value,
                        !field.not_null(),
                        false,
                    ),
                    MySQLField::Set(f) => FormItem::new_multi_select(
                        field.name().to_string(),
                        f.options.clone(),
                        if let Some(vals) = value {
                            vals.replace(",", "")
                                .split(',')
                                .map(String::from)
                                .collect::<Vec<String>>()
                        } else {
                            vec![]
                        },
                        !field.not_null(),
                        false,
                    ),
                    MySQLField::Int(f)
                    | MySQLField::BigInt(f)
                    | MySQLField::Integer(f)
                    | MySQLField::MediumInt(f)
                    | MySQLField::SmallInt(f)
                    | MySQLField::TinyInt(f) => FormItem::new_input(
                        field.name().to_string(),
                        value.as_deref(),
                        !field.not_null() || f.auto_increment || field.default_value().is_some(),
                        !field.not_null(),
                        false,
                    ),
                    MySQLField::Float(f) | MySQLField::Double(f) | MySQLField::Real(f) => {
                        FormItem::new_input(
                            field.name().to_string(),
                            value.as_deref(),
                            !field.not_null()
                                || f.auto_increment
                                || field.default_value().is_some(),
                            !field.not_null(),
                            false,
                        )
                    }
                    MySQLField::VarChar(_)
                    | MySQLField::Char(_)
                    | MySQLField::Numeric(_)
                    | MySQLField::Decimal(_)
                    | MySQLField::Year(_)
                    | MySQLField::Date(_)
                    | MySQLField::Time(_)
                    | MySQLField::DateTime(_)
                    | MySQLField::Timestamp(_) => FormItem::new_input(
                        field.name().to_string(),
                        value.as_deref(),
                        !field.not_null() || field.default_value().is_some(),
                        !field.not_null(),
                        false,
                    ),
                    MySQLField::Text(_)
                    | MySQLField::TinyText(_)
                    | MySQLField::MediumText(_)
                    | MySQLField::LongText(_)
                    | MySQLField::Json(_) => FormItem::new_input(
                        field.name().to_string(),
                        value.as_deref(),
                        !field.not_null(),
                        !field.not_null(),
                        false,
                    ),
                    MySQLField::Binary(_) | MySQLField::VarBinary(_) | MySQLField::Bit(_) => {
                        FormItem::new_input(
                            field.name().to_string(),
                            value.as_deref(),
                            !field.not_null() || field.default_value().is_some(),
                            !field.not_null(),
                            false,
                        )
                    }
                    _ => FormItem::new_input(
                        field.name().to_string(),
                        value.as_deref(),
                        !field.not_null(),
                        !field.not_null(),
                        true,
                    ),
                }
            })
            .collect();

        self.form.set_items(items.clone());
        self.old_form.set_items(items)
    }
    pub fn set_pg_fields_and_row(
        &mut self,
        fields: &[PGField],
        row: Option<&PgRow>,
        is_readonly: bool,
    ) {
        let items: Vec<FormItem<'a>> = fields
            .iter()
            .map(|field| match field.kind() {
                PGFieldKind::Int2 | PGFieldKind::Int4 | PGFieldKind::Int8 => {
                    let value = row.map(|row| get_pg_field_value(field, row));

                    FormItem::new_input(
                        field.name().to_string(),
                        value.as_deref(),
                        !field.not_null() || field.default_value().is_some(),
                        !field.not_null(),
                        field.key() && is_readonly,
                    )
                }
                _ => {
                    let value = row.map(|row| get_pg_field_value(field, row));

                    FormItem::new_input(
                        field.name().to_string(),
                        value.as_deref(),
                        !field.not_null() || field.default_value().is_some(),
                        !field.not_null(),
                        field.key() && is_readonly,
                    )
                }
            })
            .collect();

        self.form.set_items(items.clone());
        self.old_form.set_items(items)
    }

    pub fn draw<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        let bounds = f.size();
        let width = min(bounds.width - 2, 60);
        let height = min(self.form.height(), bounds.height - 2);
        let left = (bounds.width - width) / 2;
        let top = (bounds.height - height) / 2;

        let rect = Rect::new(left, top, width, height);
        f.render_widget(Clear, rect);

        self.form.draw(f, rect);
    }

    pub fn handle_event(
        &mut self,
        key: &Key,
    ) -> Result<DialogResult<HashMap<String, Option<String>>>> {
        let result = self.form.handle_event(key)?;
        match result {
            DialogResult::Confirm(map) => {
                let old_map = self.old_form.get_data();
                if map == old_map {
                    Err(Error::msg("no data changed"))
                } else {
                    let new_map = map
                        .iter()
                        .filter(|(key, val)| {
                            if let Some(v) = old_map.get(*key) {
                                v != *val
                            } else {
                                false
                            }
                        })
                        .map(|(key, val)| (key.to_string(), val.as_ref().map(|v| v.to_string())))
                        .collect();
                    Ok(DialogResult::Confirm(new_map))
                }
            }
            _ => Ok(result),
        }
    }

    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
}
