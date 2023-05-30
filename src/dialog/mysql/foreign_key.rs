use crate::{
    app::DialogResult,
    component::Command,
    event::Key,
    model::mysql::{
        get_mysql_db_names, get_mysql_field_names, get_mysql_table_names, Connections, Field,
        ForeignKey, OnDeleteKind, OnUpdateKind,
    },
    pool::{get_mysql_pool, MySQLPools},
    widget::{Form, FormItem},
};
use anyhow::Result;
use std::{cell::RefCell, cmp::min, collections::HashMap, rc::Rc};
use strum::IntoEnumIterator;
use tui::{backend::Backend, layout::Rect, widgets::Clear, Frame};
use uuid::Uuid;

pub struct ForeignKeyDialog<'a> {
    id: Option<Uuid>,
    form: Form<'a>,
    conn_id: Uuid,
    conns: Rc<RefCell<Connections>>,
    pools: Rc<RefCell<MySQLPools>>,
}

impl<'a> ForeignKeyDialog<'a> {
    pub async fn new(
        fields: &[Field],
        foreign_key: Option<&ForeignKey>,
        conn_id: &Uuid,
        conns: Rc<RefCell<Connections>>,
        pools: Rc<RefCell<MySQLPools>>,
    ) -> Result<ForeignKeyDialog<'a>> {
        let pool = get_mysql_pool(conns.clone(), pools.clone(), conn_id, None).await?;
        let ref_dbs = get_mysql_db_names(&pool).await?;

        let mut form = Form::default();
        form.set_title("Edit Foreign Key".to_string());
        form.set_items(if let Some(f) = foreign_key {
            vec![
                FormItem::new_input("name".to_string(), Some(f.name()), false, false, false),
                FormItem::new_select(
                    "field".to_string(),
                    fields
                        .iter()
                        .map(|f| f.name().to_string())
                        .collect::<Vec<String>>(),
                    Some(f.field().to_string()),
                    false,
                    false,
                ),
                FormItem::new_select(
                    "reference db".to_string(),
                    ref_dbs,
                    Some(f.ref_db().to_string()),
                    false,
                    false,
                ),
                FormItem::new_select(
                    "reference table".to_string(),
                    vec![],
                    Some(f.ref_table().to_string()),
                    false,
                    false,
                ),
                FormItem::new_select(
                    "reference field".to_string(),
                    vec![],
                    Some(f.ref_field().to_string()),
                    false,
                    false,
                ),
                FormItem::new_select(
                    "on delete".to_string(),
                    OnDeleteKind::iter().map(|s| s.to_string()).collect(),
                    f.on_delete().map(|s| s.to_string()),
                    true,
                    false,
                ),
                FormItem::new_select(
                    "on update".to_string(),
                    OnUpdateKind::iter().map(|s| s.to_string()).collect(),
                    f.on_update().map(|s| s.to_string()),
                    true,
                    false,
                ),
            ]
        } else {
            vec![
                FormItem::new_input("name".to_string(), None, false, false, false),
                FormItem::new_select(
                    "field".to_string(),
                    fields
                        .iter()
                        .map(|f| f.name().to_string())
                        .collect::<Vec<String>>(),
                    None,
                    false,
                    false,
                ),
                FormItem::new_select("reference db".to_string(), ref_dbs, None, false, false),
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

        Ok(ForeignKeyDialog {
            id: foreign_key.map(|f| f.id().to_owned()),
            form,
            conn_id: *conn_id,
            conns,
            pools,
        })
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
    pub fn get_commands(&self) -> Vec<Command> {
        self.form.get_commands()
    }
    pub fn get_ref_db(&self) -> Option<String> {
        match self.form.get_item("reference db").unwrap() {
            FormItem::Select { selected, .. } => selected.clone(),
            _ => None,
        }
    }
    pub fn set_ref_tables(&mut self, table_names: &[String]) {
        self.form.set_item(
            "reference table",
            FormItem::new_select(
                "reference table".to_string(),
                table_names.to_vec(),
                None,
                false,
                false,
            ),
        );
    }
    pub fn set_ref_fields(&mut self, field_names: &[String]) {
        self.form.set_item(
            "reference field",
            FormItem::new_multi_select(
                "reference field".to_string(),
                field_names.to_vec(),
                vec![],
                false,
                false,
            ),
        );
    }
    pub async fn handle_event(
        &mut self,
        key: &Key,
    ) -> Result<DialogResult<HashMap<String, Option<String>>>> {
        let result = self.form.handle_event(key)?;
        match result {
            DialogResult::Confirm(mut map) => {
                if let Some(id) = self.get_id() {
                    map.insert("id".to_string(), Some(id.to_string()));
                }
                Ok(DialogResult::Confirm(map))
            }
            DialogResult::Changed(name, selected) => {
                match name.as_str() {
                    "reference db" => {
                        let pool = get_mysql_pool(
                            self.conns.clone(),
                            self.pools.clone(),
                            &self.conn_id,
                            Some("information_schema"),
                        )
                        .await?;

                        let table_names = get_mysql_table_names(&pool, selected.as_str()).await?;
                        self.set_ref_tables(&table_names);
                    }
                    "reference table" => {
                        let ref_db = self.get_ref_db();
                        let pool = get_mysql_pool(
                            self.conns.clone(),
                            self.pools.clone(),
                            &self.conn_id,
                            ref_db.as_deref(),
                        )
                        .await?;
                        let fields = get_mysql_field_names(&pool, selected.as_str()).await?;
                        self.set_ref_fields(&fields);
                    }
                    _ => (),
                }
                Ok(DialogResult::Done)
            }
            _ => Ok(result),
        }
    }
}
