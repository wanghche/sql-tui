use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::pg::{Field, ForeignKey, OnDeleteKind, OnUpdateKind},
    widget::{Form, FormItem},
};
use anyhow::Result;
use std::{cmp::min, collections::HashMap};
use strum::IntoEnumIterator;
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};
use uuid::Uuid;

pub struct ForeignKeyDialog<'a> {
    id: Option<Uuid>,
    form: Form<'a>,
}
impl<'a> ForeignKeyDialog<'a> {
    pub fn new(fields: &[Field], schemas: &[String], foreign_key: Option<&ForeignKey>) -> Self {
        let mut form = Form::default();
        form.set_title("Edit Foreign Key".to_string());
        form.set_items(if let Some(fk) = foreign_key {
            vec![
                FormItem::new_input("name".to_string(), Some(fk.name()), false, false, false),
                FormItem::new_select(
                    "fields".to_string(),
                    fields.iter().map(|f| f.name.to_string()).collect(),
                    Some(fk.field().to_string()),
                    false,
                    false,
                ),
                FormItem::new_select(
                    "reference schema".to_string(),
                    schemas.to_vec(),
                    Some(fk.ref_schema().to_string()),
                    false,
                    false,
                ),
                FormItem::new_select(
                    "reference table".to_string(),
                    vec![],
                    Some(fk.ref_table().to_string()),
                    false,
                    false,
                ),
                FormItem::new_select(
                    "reference field".to_string(),
                    vec![],
                    Some(fk.ref_field().to_string()),
                    false,
                    false,
                ),
                FormItem::new_select(
                    "on delete".to_string(),
                    OnDeleteKind::iter().map(|s| s.to_string()).collect(),
                    fk.on_delete().map(|d| d.to_string()),
                    true,
                    false,
                ),
                FormItem::new_select(
                    "on update".to_string(),
                    OnUpdateKind::iter().map(|s| s.to_string()).collect(),
                    fk.on_update().map(|u| u.to_string()),
                    true,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_select(
                    "field".to_string(),
                    fields.iter().map(|f| f.name.to_string()).collect(),
                    None,
                    false,
                    false,
                ),
                FormItem::new_select(
                    "reference schema".to_string(),
                    schemas.to_vec(),
                    None,
                    false,
                    false,
                ),
                FormItem::new_select("reference table".to_string(), vec![], None, false, false),
                FormItem::new_select("reference field".to_string(), vec![], None, false, false),
                FormItem::new_select(
                    "on delete".to_string(),
                    OnDeleteKind::iter().map(|s| s.to_string()).collect(),
                    None,
                    true,
                    false,
                ),
                FormItem::new_select(
                    "on update".to_string(),
                    OnUpdateKind::iter().map(|s| s.to_string()).collect(),
                    None,
                    true,
                    false,
                ),
            ]
        });
        ForeignKeyDialog {
            id: foreign_key.map(|f| f.id.clone()),
            form,
        }
    }
    pub fn get_id(&self) -> Option<&Uuid> {
        self.id.as_ref()
    }
    pub fn get_ref_schema(&self) -> Option<String> {
        match self.form.get_item("reference schema").unwrap() {
            FormItem::Select { selected, .. } => selected.clone(),
            _ => None,
        }
    }
    pub fn set_ref_tables(&mut self, table_names: Vec<String>) {
        self.form.set_item(
            "reference table",
            FormItem::new_select(
                "reference table".to_string(),
                table_names,
                None,
                false,
                false,
            ),
        );
    }
    pub fn set_ref_fields(&mut self, field_names: Vec<String>) {
        self.form.set_item(
            "reference field",
            FormItem::new_select(
                "reference field".to_string(),
                field_names,
                None,
                false,
                false,
            ),
        );
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
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
}
