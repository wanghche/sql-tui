use crate::{
    app::DialogResult,
    event::Key,
    model::pg::{Field, FiresKind, ForEachKind, Schema, Trigger},
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
    pub fn new(fields: &Vec<Field>, schemas: &Vec<Schema>, trigger: Option<&Trigger>) -> Self {
        let mut form = Form::default();
        form.set_title("Edit Trigger".to_string());
        form.set_items(if let Some(f) = trigger {
            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_select(
                    "for each".to_string(),
                    ForEachKind::iter().map(|s| s.to_string()).collect(),
                    f.for_each().map(|s| s.to_string()),
                    true,
                    false,
                ),
                FormItem::new_select(
                    "fires".to_string(),
                    FiresKind::iter().map(|s| s.to_string()).collect(),
                    f.fires().map(|s| s.to_string()),
                    true,
                    false,
                ),
                FormItem::new_check("insert".to_string(), f.insert(), false),
                FormItem::new_check("update".to_string(), f.update(), false),
                FormItem::new_check("delete".to_string(), f.delete(), false),
                FormItem::new_check("truncate".to_string(), f.truncate(), false),
                FormItem::new_multi_select(
                    "update fields".to_string(),
                    fields.iter().map(|f| f.name().to_string()).collect(),
                    f.update_fields()
                        .iter()
                        .filter(|f| f.is_some())
                        .map(|f| f.as_deref().unwrap().to_string())
                        .collect(),
                    true,
                    false,
                ),
                FormItem::new_check("enable".to_string(), f.enable(), false),
                FormItem::new_textarea(
                    "where".to_string(),
                    f.where_condition(),
                    true,
                    false,
                    false,
                ),
                FormItem::new_select(
                    "function schema".to_string(),
                    schemas.iter().map(|s| s.name().to_string()).collect(),
                    Some(f.fn_schema().to_string()),
                    false,
                    false,
                ),
                FormItem::new_select(
                    "function name".to_string(),
                    vec![],
                    Some(f.fn_name().to_string()),
                    false,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_select(
                    "for each".to_string(),
                    ForEachKind::iter().map(|s| s.to_string()).collect(),
                    None,
                    true,
                    false,
                ),
                FormItem::new_select(
                    "fires".to_string(),
                    FiresKind::iter().map(|s| s.to_string()).collect(),
                    None,
                    true,
                    false,
                ),
                FormItem::new_check("insert".to_string(), false, false),
                FormItem::new_check("update".to_string(), false, false),
                FormItem::new_check("delete".to_string(), false, false),
                FormItem::new_check("truncate".to_string(), false, false),
                FormItem::new_multi_select(
                    "update fields".to_string(),
                    fields.iter().map(|f| f.name().to_string()).collect(),
                    vec![],
                    true,
                    false,
                ),
                FormItem::new_check("enable".to_string(), false, false),
                FormItem::new_textarea("where".to_string(), None, true, false, false),
                FormItem::new_select(
                    "function schema".to_string(),
                    schemas.iter().map(|s| s.name().to_string()).collect(),
                    None,
                    false,
                    false,
                ),
                FormItem::new_select("function name".to_string(), vec![], None, false, false),
            ]
        });

        TriggerDialog {
            id: trigger.map(|t| t.id.clone()),
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
}
