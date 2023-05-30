use crate::{
    app::DialogResult,
    component::Command,
    event::{config::*, Key},
    model::mysql::{get_mysql_db_names, get_mysql_table_names, Connections, Privilege},
    pool::{get_mysql_pool, MySQLPools},
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
    pools: Rc<RefCell<MySQLPools>>,
}

impl<'a> PrivilegeDialog<'a> {
    pub async fn new(
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<MySQLPools>>,
        conn_id: Uuid,
        privilege: Option<&Privilege>,
    ) -> Result<PrivilegeDialog<'a>> {
        let mut form = Form::default();
        form.set_title("Edit Privilege".to_string());
        let pool = get_mysql_pool(conns.clone(), pools.clone(), &conn_id, None).await?;
        let dbs = get_mysql_db_names(&pool).await?;
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
                    "name".to_string(),
                    vec![],
                    Some(p.name.clone()),
                    false,
                    false,
                ),
                FormItem::new_check("alter".to_string(), p.alter, false),
                FormItem::new_check("create".to_string(), p.create, false),
                FormItem::new_check("create view".to_string(), p.create_view, false),
                FormItem::new_check("delete".to_string(), p.delete, false),
                FormItem::new_check("drop".to_string(), p.drop, false),
                FormItem::new_check("grant option".to_string(), p.grant_option, false),
                FormItem::new_check("index".to_string(), p.index, false),
                FormItem::new_check("insert".to_string(), p.insert, false),
                FormItem::new_check("references".to_string(), p.references, false),
                FormItem::new_check("select".to_string(), p.select, false),
                FormItem::new_check("show view".to_string(), p.show_view, false),
                FormItem::new_check("trigger".to_string(), p.trigger, false),
                FormItem::new_check("update".to_string(), p.update, false),
            ]
        } else {
            vec![
                FormItem::new_select("database".to_string(), dbs, None, false, false),
                FormItem::new_select("name".to_string(), vec![], None, false, false),
                FormItem::new_check("alter".to_string(), false, false),
                FormItem::new_check("create".to_string(), false, false),
                FormItem::new_check("create view".to_string(), false, false),
                FormItem::new_check("delete".to_string(), false, false),
                FormItem::new_check("drop".to_string(), false, false),
                FormItem::new_check("grant option".to_string(), false, false),
                FormItem::new_check("index".to_string(), false, false),
                FormItem::new_check("insert".to_string(), false, false),
                FormItem::new_check("references".to_string(), false, false),
                FormItem::new_check("select".to_string(), false, false),
                FormItem::new_check("show view".to_string(), false, false),
                FormItem::new_check("trigger".to_string(), false, false),
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
                if key.as_str() == "database" {
                    let pool = get_mysql_pool(
                        self.conns.clone(),
                        self.pools.clone(),
                        &self.conn_id,
                        Some("information_schema"),
                    )
                    .await?;
                    let tables = get_mysql_table_names(&pool, &val).await?;
                    if let FormItem::Select { options, .. } =
                        self.form.get_item_mut("name").unwrap()
                    {
                        *options = tables;
                    }
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
        let mut cmds = self.form.get_commands();
        cmds.extend(vec![
            Command {
                name: "Cancel",
                key: CANCEL_KEY,
            },
            Command {
                name: "Ok",
                key: SAVE_KEY,
            },
        ]);
        cmds
    }
}
