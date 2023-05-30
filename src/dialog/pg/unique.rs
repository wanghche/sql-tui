use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::pg::{Field, Unique},
    widget::{Form, FormItem},
};
use anyhow::Result;
use std::{cmp::min, collections::HashMap};
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};
use uuid::Uuid;

pub struct UniqueDialog<'a> {
    id: Option<Uuid>,
    form: Form<'a>,
}

impl<'a> UniqueDialog<'a> {
    pub fn new(fields: &Vec<Field>, unique: Option<&Unique>) -> Self {
        let mut form = Form::default();
        form.set_title("Edit Unique".to_string());
        form.set_items(if let Some(u) = unique {
            vec![
                FormItem::new_input("name".to_string(), Some(u.name()), false, false, false),
                FormItem::new_multi_select(
                    "fields".to_string(),
                    fields.iter().map(|f| f.name().to_string()).collect(),
                    u.fields().to_vec(),
                    false,
                    false,
                ),
                FormItem::new_input("comment".to_string(), u.comment(), true, false, false),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_multi_select(
                    "fields".to_string(),
                    fields.iter().map(|f| f.name().to_string()).collect(),
                    vec![],
                    false,
                    false,
                ),
                FormItem::new_input("comment".to_string(), None, true, false, false),
            ]
        });

        UniqueDialog {
            id: unique.map(|u| u.id.to_owned()),
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
        let rect = Rect::new(left, top, width, height as u16);
        f.render_widget(Clear, rect);
        self.form.draw(f, rect);
    }
    pub fn get_id(&self) -> Option<&Uuid> {
        self.id.as_ref()
    }
    pub fn handle_event(
        &mut self,
        key: &Key,
    ) -> Result<DialogResult<HashMap<String, Option<String>>>> {
        let r = self.form.handle_event(key)?;
        match r {
            DialogResult::Confirm(mut map) => {
                if let Some(id) = self.id.as_ref() {
                    map.insert("id".to_string(), Some(id.to_string()));
                }
                Ok(DialogResult::Confirm(map))
            }
            _ => Ok(r),
        }
    }
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
}
