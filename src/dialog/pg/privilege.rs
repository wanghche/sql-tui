use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::pg::{get_pg_db_names, get_pg_schemas, get_pg_table_names, Connections, Privilege},
    pool::{get_pg_pool, PGPools},
    widget::{Form, FormItem},
};
use anyhow::Result;
use std::{cell::RefCell, rc::Rc};
use std::{cmp::min, collections::HashMap};
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};
use uuid::Uuid;

pub struct PrivilegeDialog<'a> {
    id: Option<Uuid>,
    form: Form<'a>,
    conn_id: Uuid,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<PGPools>>,
}

impl<'a> PrivilegeDialog<'a> {
    pub async fn new(
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<PGPools>>,
        conn_id: Uuid,
        privilege: Option<&Privilege>,
    ) -> Result<PrivilegeDialog<'a>> {
        let mut form = Form::default();
        form.set_title("Edit Privilege".to_string());
        let pool = get_pg_pool(conns.clone(), pools.clone(), &conn_id, None).await?;
        let dbs = get_pg_db_names(&pool).await?;
        form.set_items(if let Some(p) = privilege {
            vec![
                FormItem::new_select(
                    "database".to_string(),
                    dbs,
                    Some(p.db.clone()),
                    false,
                    false,
                ),
                FormItem::new_select(
                    "schema".to_string(),
                    vec![],
                    Some(p.schema.clone()),
                    false,
                    false,
                ),
                FormItem::new_select(
                    "name".to_string(),
                    vec![],
                    Some(p.name.clone()),
                    false,
                    false,
                ),
                FormItem::new_check("delete".to_string(), p.delete, false),
                FormItem::new_check("insert".to_string(), p.insert, false),
                FormItem::new_check("references".to_string(), p.references, false),
                FormItem::new_check("select".to_string(), p.select, false),
                FormItem::new_check("trigger".to_string(), p.trigger, false),
                FormItem::new_check("truncate".to_string(), p.truncate, false),
                FormItem::new_check("update".to_string(), p.update, false),
            ]
        } else {
            vec![
                FormItem::new_select("database".to_string(), dbs, None, false, false),
                FormItem::new_select("schema".to_string(), vec![], None, false, false),
                FormItem::new_select("name".to_string(), vec![], None, false, false),
                FormItem::new_check("delete".to_string(), false, false),
                FormItem::new_check("insert".to_string(), false, false),
                FormItem::new_check("references".to_string(), false, false),
                FormItem::new_check("select".to_string(), false, false),
                FormItem::new_check("trigger".to_string(), false, false),
                FormItem::new_check("truncate".to_string(), false, false),
                FormItem::new_check("update".to_string(), false, false),
            ]
        });
        Ok(PrivilegeDialog {
            id: privilege.map(|p| p.id),
            form,
            conns,
            conn_id,
            pools,
        })
    }
    pub fn get_id(&self) -> Option<&Uuid> {
        self.id.as_ref()
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
        let form_result = self.form.handle_event(key)?;
        match form_result {
            DialogResult::Changed(key, val) => {
                match key.as_str() {
                    "database" => {
                        let schemas = get_pg_schemas(
                            self.conns.clone(),
                            self.pools.clone(),
                            &self.conn_id,
                            Some(&val),
                        )
                        .await?;
                        if let FormItem::Select { options, .. } =
                            self.form.get_item_mut("schema").unwrap()
                        {
                            *options = schemas.iter().map(|s| s.name().to_string()).collect();
                        }
                    }
                    "schema" => {
                        let db = self.form.get_value("database");
                        let pool = get_pg_pool(
                            self.conns.clone(),
                            self.pools.clone(),
                            &self.conn_id,
                            db.as_deref(),
                        )
                        .await?;
                        let tables = get_pg_table_names(&pool, &val).await?;
                        if let FormItem::Select { options, .. } =
                            self.form.get_item_mut("name").unwrap()
                        {
                            *options = tables;
                        }
                    }
                    _ => (),
                }
                Ok(DialogResult::Done)
            }
            DialogResult::Confirm(mut map) => {
                if let Some(id) = self.id.as_ref() {
                    map.insert("id".to_string(), Some(id.to_string()));
                }
                Ok(DialogResult::Confirm(map))
            }

            _ => Ok(form_result),
        }
    }
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
}
