use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::mysql::Check,
    widget::{Form, FormItem},
};
use anyhow::Result;
use std::{cmp::min, collections::HashMap};
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};
use uuid::Uuid;

pub struct CheckDialog<'a> {
    id: Option<Uuid>,
    form: Form<'a>,
}

impl<'a> CheckDialog<'a> {
    pub fn new(check: Option<&Check>) -> Self {
        let mut form = Form::default();
        form.set_title("Edit Check".to_string());
        form.set_items(if let Some(c) = check {
            vec![
                FormItem::new_input("name".to_string(), Some(c.name()), false, false, false),
                FormItem::new_textarea(
                    "expression".to_string(),
                    Some(c.expression()),
                    false,
                    false,
                    false,
                ),
                FormItem::new_check("not enforced".to_string(), c.not_enforced(), false),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_textarea("expression".to_string(), None, false, false, false),
                FormItem::new_check("not enforced".to_string(), false, false),
            ]
        });

        CheckDialog {
            id: check.map(|c| c.id().to_owned()),
            form,
        }
    }
    pub fn get_id(&self) -> Option<&Uuid> {
        self.id.as_ref()
    }
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
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
        let r = self.form.handle_event(key)?;
        match r {
            DialogResult::Confirm(mut map) => {
                if let Some(id) = self.id {
                    map.insert("id".to_string(), Some(id.to_string()));
                }
                Ok(DialogResult::Confirm(map))
            }
            _ => Ok(r),
        }
    }
}
