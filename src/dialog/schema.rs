use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::pg::Schema,
    widget::{Form, FormItem},
};
use anyhow::Result;
use std::{cmp::min, collections::HashMap};
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};

pub enum Mode {
    Create,
    Edit,
}

pub struct SchemaDialog<'a> {
    mode: Mode,
    form: Form<'a>,
}

impl<'a> SchemaDialog<'a> {
    pub fn new(owners: Vec<String>, schema: Option<&Schema>) -> Self {
        let mut form = Form::default();
        form.set_title(if let Some(s) = schema {
            format!("Edit {}", s.name())
        } else {
            "New Schema".to_string()
        });
        form.set_items(if let Some(s) = schema {
            vec![
                FormItem::new_input("name".to_string(), Some(s.name()), false, false, false),
                FormItem::new_select(
                    "owner".to_string(),
                    owners,
                    s.owner().map(|o| o.to_string()),
                    true,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_select("owner".to_string(), owners, None, true, false),
            ]
        });
        SchemaDialog {
            mode: if schema.is_some() {
                Mode::Edit
            } else {
                Mode::Create
            },
            form,
        }
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
        self.form.handle_event(key)
    }
    pub fn get_mode(&self) -> &Mode {
        &self.mode
    }
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
}
