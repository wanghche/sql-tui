use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::pg::{DoInstead, EventKind, Rule},
    widget::{Form, FormItem},
};
use anyhow::Result;
use std::{cmp::min, collections::HashMap};
use strum::IntoEnumIterator;
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};
use uuid::Uuid;

pub struct RuleDialog<'a> {
    id: Option<Uuid>,
    form: Form<'a>,
}

impl<'a> RuleDialog<'a> {
    pub fn new(rule: Option<&Rule>) -> Self {
        let mut form = Form::default();
        form.set_items(if let Some(r) = rule {
            vec![
                FormItem::new_input("name".to_string(), Some(r.name()), false, false, false),
                FormItem::new_select(
                    "event".to_string(),
                    EventKind::iter().map(|s| s.to_string()).collect(),
                    Some(r.event().to_string()),
                    false,
                    false,
                ),
                FormItem::new_select(
                    "do instead".to_string(),
                    DoInstead::iter().map(|s| s.to_string()).collect(),
                    r.do_instead().map(|s| s.to_string()),
                    true,
                    false,
                ),
                FormItem::new_textarea(
                    "where".to_string(),
                    r.where_condition(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_textarea(
                    "definition".to_string(),
                    r.definition(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_check("enable".to_string(), r.enable(), false),
                FormItem::new_input("comment".to_string(), r.comment(), true, false, false),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_select(
                    "event".to_string(),
                    EventKind::iter().map(|s| s.to_string()).collect(),
                    None,
                    false,
                    false,
                ),
                FormItem::new_select(
                    "do instead".to_string(),
                    DoInstead::iter().map(|s| s.to_string()).collect(),
                    None,
                    true,
                    false,
                ),
                FormItem::new_textarea("where".to_string(), None, true, false, false),
                FormItem::new_textarea("definition".to_string(), None, true, false, false),
                FormItem::new_check("enable".to_string(), true, false),
                FormItem::new_input("comment".to_string(), None, true, false, false),
            ]
        });
        RuleDialog {
            id: rule.map(|r| r.id.clone()),
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
