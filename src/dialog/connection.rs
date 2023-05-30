use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::{
        mysql::Connection as MySQLConnection, pg::Connection as PGConnection, Connect, DatabaseKind,
    },
    widget::{Form, FormItem},
};
use anyhow::Result;
use std::{cmp::min, collections::HashMap};
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};
use uuid::Uuid;

#[derive(Default)]
pub struct ConnectionDialog<'a> {
    id: Option<Uuid>,
    kind: DatabaseKind,
    form: Form<'a>,
}

impl<'a> ConnectionDialog<'a> {
    pub fn set_mysql_connection(&mut self, conn: Option<&MySQLConnection>) {
        self.id = conn.map(|c| *c.get_id());
        self.kind = DatabaseKind::MySQL;
        let mut form = self.create_mysql_form(conn);
        form.set_title(if let Some(conn) = conn {
            format!("Edit {}", conn.name)
        } else {
            "New Connection".to_string()
        });
        self.form = form;
    }
    pub fn set_pg_connection(&mut self, conn: Option<&PGConnection>) {
        self.id = conn.map(|c| *c.get_id());
        self.kind = DatabaseKind::PostgreSQL;
        let mut form = self.create_pg_form(conn);
        form.set_title(if let Some(conn) = conn {
            format!("Edit {}", conn.name)
        } else {
            "New Connection".to_string()
        });
        self.form = form
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
        let result = self.form.handle_event(key)?;
        if let DialogResult::Confirm(mut map) = result {
            map.insert("kind".to_string(), Some(self.kind.to_string()));
            if let Some(id) = self.id {
                map.insert("id".to_string(), Some(id.to_string()));
            }
            Ok(DialogResult::Confirm(map))
        } else {
            Ok(result)
        }
    }
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
    fn create_mysql_form(&mut self, conn: Option<&MySQLConnection>) -> Form<'a> {
        let mut form = Form::default();
        form.set_items(if let Some(conn) = conn {
            vec![
                FormItem::new_input(
                    "name".to_string(),
                    Some(conn.get_name()),
                    false,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "host".to_string(),
                    Some(conn.get_host()),
                    false,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "port".to_string(),
                    Some(conn.get_port()),
                    true,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "user".to_string(),
                    Some(conn.get_user()),
                    false,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "password".to_string(),
                    Some(conn.get_password()),
                    false,
                    false,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_input("host".to_string(), None, false, false, false),
                FormItem::new_input("port".to_string(), None, true, false, false),
                FormItem::new_input("user".to_string(), None, false, false, false),
                FormItem::new_input("password".to_string(), None, false, false, false),
            ]
        });
        form
    }
    fn create_pg_form(&mut self, conn: Option<&PGConnection>) -> Form<'a> {
        let mut form = Form::default();
        form.set_items(if let Some(conn) = conn {
            vec![
                FormItem::new_input(
                    "name".to_string(),
                    Some(conn.get_name()),
                    false,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "host".to_string(),
                    Some(conn.get_host()),
                    false,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "port".to_string(),
                    Some(conn.get_port()),
                    true,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "init db".to_string(),
                    conn.get_init_db(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "user".to_string(),
                    Some(conn.get_user()),
                    false,
                    false,
                    false,
                ),
                FormItem::new_input(
                    "password".to_string(),
                    Some(conn.get_password()),
                    false,
                    false,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_input("host".to_string(), Some("localhost"), false, false, false),
                FormItem::new_input("port".to_string(), Some("5432"), true, false, false),
                FormItem::new_input("init db".to_string(), Some("postgres"), true, false, false),
                FormItem::new_input("user".to_string(), Some("postgres"), false, false, false),
                FormItem::new_input("password".to_string(), None, false, false, false),
            ]
        });
        form
    }
}
