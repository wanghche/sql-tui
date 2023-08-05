use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::{
        mysql::{get_mysql_field_value, Field as MySQLField},
        pg::{get_pg_field_value, Field as PGField},
    },
    widget::{Form, FormItem},
};
use anyhow::Result;
use sqlx::{mysql::MySqlRow, postgres::PgRow};
use std::{cmp::min, collections::HashMap};
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};

pub struct DetailDialog<'a> {
    form: Form<'a>,
}

impl<'a> DetailDialog<'a> {
    pub fn from_map(title: String, map: &HashMap<String, Option<String>>) -> Self {
        let items: Vec<FormItem<'a>> = map
            .iter()
            .map(|(name, value)| {
                FormItem::new_input(name.to_string(), value.as_deref(), true, true, true)
            })
            .collect();
        let mut form = Form::default();
        form.set_items(items);
        form.set_title(title);

        DetailDialog { form }
    }
    pub fn from_mysql_row(title: String, fields: &[MySQLField], row: &MySqlRow) -> Self {
        let mut form = Form::default();
        form.set_title(title);
        form.set_items(
            fields
                .iter()
                .map(|field| {
                    FormItem::new_input(
                        field.name().to_string(),
                        get_mysql_field_value(field, row).as_deref(),
                        true,
                        true,
                        true,
                    )
                })
                .collect(),
        );
        DetailDialog { form }
    }
    pub fn from_pg_row(title: String, fields: &[PGField], row: &PgRow) -> Self {
        let mut form = Form::default();
        form.set_title(title);
        form.set_items(
            fields
                .iter()
                .map(|field| {
                    FormItem::new_input(
                        field.name().to_string(),
                        Some(&get_pg_field_value(field, row)),
                        true,
                        true,
                        true,
                    )
                })
                .collect(),
        );
        DetailDialog { form }
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
        self.form.handle_event(key)
    }

    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
}
