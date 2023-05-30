use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::pg::RoleMember,
    widget::{Form, FormItem},
};
use anyhow::Result;
use sqlx::postgres::types::Oid;
use std::{cmp::min, collections::HashMap};
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};

pub struct RoleMemberDialog<'a> {
    role_id: Option<Oid>,
    member_id: Option<Oid>,
    form: Form<'a>,
}

impl<'a> RoleMemberDialog<'a> {
    pub fn new(rm: &RoleMember) -> Self {
        let mut form = Form::default();

        form.set_title("Edit Role Member".to_string());
        form.set_items(vec![
            FormItem::new_input(
                "Role Name".to_string(),
                if rm.role_name.is_some() {
                    rm.role_name.as_deref()
                } else {
                    rm.member_name.as_deref()
                },
                false,
                false,
                true,
            ),
            FormItem::new_check("granted".to_string(), rm.granted, false),
            FormItem::new_check("admin option".to_string(), rm.admin_option, false),
        ]);
        RoleMemberDialog {
            role_id: rm.role_oid,
            member_id: rm.member_oid,
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
    pub fn handle_event(
        &mut self,
        key: &Key,
    ) -> Result<DialogResult<HashMap<String, Option<String>>>> {
        let result = self.form.handle_event(key)?;
        match result {
            DialogResult::Confirm(mut map) => {
                if let Some(oid) = self.role_id.as_ref() {
                    map.insert("role_oid".to_string(), Some(oid.0.to_string()));
                }
                if let Some(oid) = self.member_id.as_ref() {
                    map.insert("member_oid".to_string(), Some(oid.0.to_string()));
                }
                Ok(DialogResult::Confirm(map))
            }
            _ => Ok(result),
        }
    }
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
}
