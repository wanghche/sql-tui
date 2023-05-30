use crate::{
    app::DialogResult,
    component::Command,
    event::{config::*, Key},
    model::mysql::{Trigger, TriggerAction, TriggerTime},
    widget::{Form, FormItem},
};
use anyhow::Result;
use std::{cmp::min, collections::HashMap};
use strum::IntoEnumIterator;
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};
use uuid::Uuid;

pub struct TriggerDialog<'a> {
    id: Option<Uuid>,
    form: Form<'a>,
}

impl<'a> TriggerDialog<'a> {
    pub fn new(trigger: Option<&Trigger>) -> Self {
        let mut form = Form::default();
        form.set_title("Edit Trigger".to_string());
        form.set_items(if let Some(f) = trigger {
            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_select(
                    "time".to_string(),
                    TriggerTime::iter().map(|s| s.to_string()).collect(),
                    Some(f.time().to_string()),
                    false,
                    false,
                ),
                FormItem::new_select(
                    "action".to_string(),
                    TriggerAction::iter().map(|s| s.to_string()).collect(),
                    Some(f.action().to_string()),
                    false,
                    false,
                ),
                FormItem::new_textarea(
                    "statement".to_string(),
                    Some(f.statement()),
                    false,
                    false,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_select(
                    "time".to_string(),
                    TriggerTime::iter().map(|s| s.to_string()).collect(),
                    None,
                    false,
                    false,
                ),
                FormItem::new_select(
                    "action".to_string(),
                    TriggerAction::iter().map(|s| s.to_string()).collect(),
                    None,
                    false,
                    false,
                ),
                FormItem::new_textarea("statement".to_string(), None, false, false, false),
            ]
        });

        TriggerDialog {
            id: trigger.map(|t| t.id().to_owned()),
            form,
        }
    }
    pub fn get_id(&self) -> Option<&Uuid> {
        self.id.as_ref()
    }
    pub fn get_commands(&self) -> Vec<Command> {
        let mut cmds = self.form.get_commands();
        cmds.extend(vec![
            Command {
                name: "Cancel",
                key: CANCEL_KEY,
            },
            Command {
                name: "Ok",
                key: CONFIRM_KEY,
            },
        ]);
        cmds
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
                if let Some(id) = self.id.as_ref() {
                    map.insert("id".to_string(), Some(id.to_string()));
                }
                Ok(DialogResult::Confirm(map))
            }
            _ => Ok(r),
        }
    }
}
