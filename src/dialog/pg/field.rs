use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::pg::{Field, FieldKind},
    widget::{Form, FormItem},
};
use anyhow::Result;
use std::{cmp::min, collections::HashMap};
use strum::IntoEnumIterator;
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};
use uuid::Uuid;

pub struct FieldDialog<'a> {
    id: Option<Uuid>,
    form: Form<'a>,
}

impl<'a> FieldDialog<'a> {
    pub fn new(field: Option<&Field>) -> Self {
        let mut form = Form::default();
        form.set_title("Edit Field".to_string());
        let items = if let Some(f) = field {
            let mut items = vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_select(
                    "type".to_string(),
                    FieldKind::iter().map(|s| s.to_string()).collect(),
                    Some(f.kind().to_string()),
                    false,
                    true,
                ),
                FormItem::new_check("not null".to_string(), f.not_null(), false),
                FormItem::new_check("key".to_string(), f.key(), false),
                FormItem::new_input(
                    "default value".to_string(),
                    f.default_value(),
                    true,
                    false,
                    false,
                ),
            ];
            match f.kind {
                FieldKind::BigSerial
                | FieldKind::Serial
                | FieldKind::Serial2
                | FieldKind::Serial8
                | FieldKind::SmallSerial => {
                    items.pop();
                }
                FieldKind::VarChar
                | FieldKind::Char
                | FieldKind::Interval
                | FieldKind::Time
                | FieldKind::Timestamp
                | FieldKind::TimestampTz
                | FieldKind::TimeTz
                | FieldKind::VarBit
                | FieldKind::Bit => {
                    items.push(FormItem::new_input(
                        "length".to_string(),
                        f.length().as_deref(),
                        true,
                        false,
                        false,
                    ));
                }
                FieldKind::Decimal | FieldKind::Numeric => {
                    items.push(FormItem::new_input(
                        "length".to_string(),
                        f.length().as_deref(),
                        true,
                        false,
                        false,
                    ));
                    items.push(FormItem::new_input(
                        "decimal".to_string(),
                        f.decimal().as_deref(),
                        true,
                        false,
                        false,
                    ));
                }
                _ => (),
            }
            items.push(FormItem::new_input(
                "comment".to_string(),
                f.comment(),
                true,
                false,
                false,
            ));
            items
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_select(
                    "type".to_string(),
                    FieldKind::iter().map(|s| s.to_string()).collect(),
                    Some(FieldKind::default().to_string()),
                    false,
                    false,
                ),
                FormItem::new_check("not null".to_string(), false, false),
                FormItem::new_check("key".to_string(), false, false),
                FormItem::new_input("default value".to_string(), None, true, false, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
            ]
        };
        form.set_items(items);
        FieldDialog {
            id: field.map(|f| f.id.to_owned()),
            form,
        }
    }
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
    pub fn get_id(&self) -> Option<&Uuid> {
        self.id.as_ref()
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

    pub fn handle_event(
        &mut self,
        key: &Key,
    ) -> Result<DialogResult<HashMap<String, Option<String>>>> {
        let event_result = self.form.handle_event(key)?;
        match event_result {
            DialogResult::Changed(name, value) => {
                match name.as_str() {
                    "type" => {
                        let kind = FieldKind::try_from(value.as_str()).unwrap();
                        let mut items = vec![
                            FormItem::new_input(
                                "name".to_string(),
                                self.form.get_item("name").unwrap().get_value().as_deref(),
                                false,
                                false,
                                false,
                            ),
                            FormItem::new_select(
                                "type".to_string(),
                                FieldKind::iter().map(|s| s.to_string()).collect(),
                                Some(value),
                                false,
                                false,
                            ),
                            FormItem::new_check("not null".to_string(), false, false),
                            FormItem::new_check("key".to_string(), false, false),
                            FormItem::new_input(
                                "default value".to_string(),
                                None,
                                true,
                                false,
                                false,
                            ),
                        ];

                        match kind {
                            FieldKind::BigSerial
                            | FieldKind::Serial
                            | FieldKind::Serial2
                            | FieldKind::Serial8
                            | FieldKind::SmallSerial => {
                                items.pop();
                            }
                            FieldKind::VarChar
                            | FieldKind::Char
                            | FieldKind::Interval
                            | FieldKind::Time
                            | FieldKind::Timestamp
                            | FieldKind::TimestampTz
                            | FieldKind::TimeTz
                            | FieldKind::VarBit
                            | FieldKind::Bit => {
                                items.push(FormItem::new_input(
                                    "length".to_string(),
                                    None,
                                    true,
                                    false,
                                    false,
                                ));
                            }
                            FieldKind::Decimal | FieldKind::Numeric => {
                                items.push(FormItem::new_input(
                                    "length".to_string(),
                                    None,
                                    true,
                                    false,
                                    false,
                                ));
                                items.push(FormItem::new_input(
                                    "decimal".to_string(),
                                    None,
                                    true,
                                    false,
                                    false,
                                ));
                            }
                            _ => (),
                        }
                        items.push(FormItem::new_input(
                            "comment".to_string(),
                            None,
                            true,
                            false,
                            false,
                        ));
                        self.form.set_items(items);
                    }
                    "key" => {
                        if value == "true" {
                            self.form.set_value("not null", "true");
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
            _ => Ok(event_result),
        }
    }
}
