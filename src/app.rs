use crate::{
    component::{
        CommandBarComponent, ConnectionListComponent, DataListComponentMySQL, DataListComponentPG,
        HomeComponent, QueryDetailComponentMySQL, QueryDetailComponentPG, QueryListComponent,
        RoleDetailComponentPG, RoleListComponentPG, TableDetailComponentMySQL,
        TableDetailComponentPG, TableListComponentMySQL, TableListComponentPG,
        UserDetailComponentMySQL, UserListComponentMySQL, ViewDetailComponentMySQL,
        ViewDetailComponentPG, ViewListComponentMySQL, ViewListComponentPG,
    },
    config::Config,
    dialog::confirm::{ConfirmDialog, Kind as ConfirmKind},
    event::{self, Key, KeyCode, KeyModifier},
    model::{
        mysql::Connections as MySQLConnections, pg::Connections as PGConnections, query::Queries,
        DatabaseKind,
    },
    pool::{MySQLPools, PGPools},
};
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{cell::RefCell, io, rc::Rc};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};
use uuid::Uuid;

pub const APP_DIR: &str = ".sqltui";

pub enum DialogResult<T> {
    Done,
    Cancel,
    Confirm(T),
    Changed(String, String),
}

pub enum Goto {
    TableListMySQL {
        conn_id: Uuid,
        db_name: String,
    },
    TableDetailMySQL {
        conn_id: Uuid,
        db_name: String,
        table_name: Option<String>,
    },
    TableListPG {
        conn_id: Uuid,
        db_name: String,
        schema_name: String,
    },
    TableDetailPG {
        conn_id: Uuid,
        db_name: String,
        schema_name: String,
        table_name: Option<String>,
    },
    QueryList {
        conn_id: Uuid,
        db_name: String,
        kind: DatabaseKind,
    },
    QueryDetailMySQL {
        conn_id: Uuid,
        db_name: String,
        query_name: Option<String>,
    },
    QueryDetailPG {
        conn_id: Uuid,
        db_name: String,
        query_name: Option<String>,
    },
    ViewListMySQL {
        conn_id: Uuid,
        db_name: String,
    },
    ViewDetailMySQL {
        conn_id: Uuid,
        db_name: String,
        view_name: Option<String>,
    },
    ViewListPG {
        conn_id: Uuid,
        db_name: String,
        schema_name: String,
    },
    ViewDetailPG {
        conn_id: Uuid,
        db_name: String,
        schema_name: String,
        view_name: Option<String>,
    },
    UserListMySQL {
        conn_id: Uuid,
    },
    UserDetailMySQL {
        conn_id: Uuid,
        user_host: Option<String>,
        user_name: Option<String>,
    },
    RoleListPG {
        conn_id: Uuid,
    },
    RoleDetailPG {
        conn_id: Uuid,
        role_name: Option<String>,
    },
    DataListMySQL {
        conn_id: Uuid,
        db_name: String,
        table_name: String,
    },
    DataListPG {
        conn_id: Uuid,
        db_name: String,
        schema_name: String,
        table_name: String,
    },
}

#[derive(PartialEq)]
pub enum Focus {
    LeftPanel,
    MainPanel,
}

pub enum ComponentResult {
    Back(MainPanel),
    BackRefresh(MainPanel),
    Done,
    Goto(Goto),
    Focus(Focus),
}

#[derive(Clone)]
pub enum MainPanel {
    Home,
    TableListMySQL,
    TableDetailMySQL,
    DataListMySQL,
    TableListPG,
    TableDetailPG,
    DataListPG,
    QueryList,
    QueryDetailMySQL,
    QueryDetailPG,
    ViewListMySQL,
    ViewDetailMySQL,
    ViewListPG,
    ViewDetailPG,
    UserListMySQL,
    UserDetailMySQL,
    RoleListPG,
    RoleDetailPG,
}

pub struct App<'a> {
    pub focus: Focus,
    pub main_panel: MainPanel,
    pub home: HomeComponent,
    pub connection_list: ConnectionListComponent<'a>,
    pub command_bar: Rc<RefCell<CommandBarComponent>>,
    pub table_list_mysql: TableListComponentMySQL,
    pub table_list_pg: TableListComponentPG,
    pub data_list_mysql: DataListComponentMySQL<'a>,
    pub data_list_pg: DataListComponentPG<'a>,
    pub table_detail_mysql: TableDetailComponentMySQL<'a>,
    pub table_detail_pg: TableDetailComponentPG<'a>,
    pub query_list: QueryListComponent,
    pub query_detail_mysql: QueryDetailComponentMySQL<'a>,
    pub query_detail_pg: QueryDetailComponentPG<'a>,
    pub view_list_mysql: ViewListComponentMySQL,
    pub view_detail_mysql: ViewDetailComponentMySQL<'a>,
    pub view_list_pg: ViewListComponentPG,
    pub view_detail_pg: ViewDetailComponentPG<'a>,
    pub user_list_mysql: UserListComponentMySQL,
    pub user_detail_mysql: UserDetailComponentMySQL<'a>,
    pub role_list_pg: RoleListComponentPG,
    pub role_detail_pg: RoleDetailComponentPG<'a>,
    pub error_dlg: Option<ConfirmDialog>,
}

impl<'a> App<'a> {
    pub fn new(
        mysql_conns: Rc<RefCell<MySQLConnections>>,
        pg_conns: Rc<RefCell<PGConnections>>,
        mysql_pools: Rc<RefCell<MySQLPools>>,
        pg_pools: Rc<RefCell<PGPools>>,
        config: Rc<RefCell<Config>>,
        queries: Rc<RefCell<Queries>>,
    ) -> Self {
        let command_bar = Rc::new(RefCell::new(CommandBarComponent::new()));
        let data_list_mysql = DataListComponentMySQL::new(
            mysql_conns.clone(),
            mysql_pools.clone(),
            command_bar.clone(),
        );
        let data_list_pg =
            DataListComponentPG::new(pg_conns.clone(), pg_pools.clone(), command_bar.clone());
        let table_detail_mysql = TableDetailComponentMySQL::new(
            command_bar.clone(),
            mysql_conns.clone(),
            mysql_pools.clone(),
        );
        let table_detail_pg =
            TableDetailComponentPG::new(command_bar.clone(), pg_conns.clone(), pg_pools.clone());
        let table_list_mysql = TableListComponentMySQL::new(
            command_bar.clone(),
            mysql_conns.clone(),
            mysql_pools.clone(),
        );
        let table_list_pg =
            TableListComponentPG::new(command_bar.clone(), pg_conns.clone(), pg_pools.clone());
        let query_detail_mysql = QueryDetailComponentMySQL::new(
            mysql_conns.clone(),
            mysql_pools.clone(),
            queries.clone(),
            command_bar.clone(),
        );
        let query_detail_pg = QueryDetailComponentPG::new(
            pg_conns.clone(),
            pg_pools.clone(),
            queries.clone(),
            command_bar.clone(),
        );
        let query_list = QueryListComponent::new(queries, command_bar.clone());
        let view_detail_mysql = ViewDetailComponentMySQL::new(
            command_bar.clone(),
            mysql_conns.clone(),
            mysql_pools.clone(),
        );
        let view_list_mysql = ViewListComponentMySQL::new(
            command_bar.clone(),
            mysql_conns.clone(),
            mysql_pools.clone(),
        );
        let view_detail_pg =
            ViewDetailComponentPG::new(command_bar.clone(), pg_conns.clone(), pg_pools.clone());
        let view_list_pg =
            ViewListComponentPG::new(command_bar.clone(), pg_conns.clone(), pg_pools.clone());

        let user_list_mysql = UserListComponentMySQL::new(
            command_bar.clone(),
            mysql_conns.clone(),
            mysql_pools.clone(),
        );
        let user_detail_mysql = UserDetailComponentMySQL::new(
            command_bar.clone(),
            mysql_conns.clone(),
            mysql_pools.clone(),
        );
        let role_list_pg =
            RoleListComponentPG::new(command_bar.clone(), pg_conns.clone(), pg_pools.clone());
        let role_detail_pg =
            RoleDetailComponentPG::new(command_bar.clone(), pg_conns.clone(), pg_pools.clone());
        let connection_list = ConnectionListComponent::new(
            command_bar.clone(),
            mysql_conns,
            pg_conns,
            mysql_pools,
            pg_pools,
            config,
        );

        App {
            focus: Focus::LeftPanel,
            main_panel: MainPanel::Home,
            home: HomeComponent::new(),
            connection_list,
            command_bar,
            table_list_mysql,
            data_list_mysql,
            table_detail_mysql,
            table_list_pg,
            data_list_pg,
            table_detail_pg,
            query_list,
            query_detail_mysql,
            query_detail_pg,
            view_list_mysql,
            view_detail_mysql,
            view_list_pg,
            view_detail_pg,
            user_list_mysql,
            user_detail_mysql,
            role_list_pg,
            role_detail_pg,
            error_dlg: None,
        }
    }
    pub async fn start(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;
        terminal.clear()?;

        let events = event::Events::new(250);

        loop {
            terminal.draw(|f| {
                self.draw_component(f).unwrap();
                self.draw_dialog(f);
            })?;
            match events.next()? {
                event::Event::Input(key) => {
                    if key
                        == (Key {
                            code: KeyCode::Char('c'),
                            modifier: KeyModifier::Ctrl,
                        })
                    {
                        break;
                    }
                    if let Some(dlg) = self.error_dlg.as_mut() {
                        match dlg.handle_event(&key) {
                            DialogResult::Done => (),
                            _ => {
                                self.error_dlg = None;
                            }
                        }
                        continue;
                    }
                    if let Err(e) = self.handle_input_event(&key).await {
                        self.error_dlg = Some(ConfirmDialog::new(
                            ConfirmKind::Error,
                            "Error",
                            e.root_cause().to_string().as_str(),
                        ));
                    };
                }
                event::Event::Tick => {}
            }
        }
        execute!(io::stdout(), LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }
    pub async fn handle_input_event(&mut self, key: &Key) -> Result<()> {
        match self.focus {
            Focus::LeftPanel => {
                match self.connection_list.handle_event(key).await? {
                    ComponentResult::Goto(goto) => match goto {
                        Goto::TableListMySQL { conn_id, db_name } => {
                            self.table_list_mysql.set_data(&conn_id, &db_name).await?;
                            self.main_panel = MainPanel::TableListMySQL;
                            self.focus = Focus::MainPanel;
                        }
                        Goto::TableListPG {
                            conn_id,
                            db_name,
                            schema_name,
                        } => {
                            self.table_list_pg
                                .set_data(&conn_id, &db_name, &schema_name)
                                .await?;
                            self.main_panel = MainPanel::TableListPG;
                            self.focus = Focus::MainPanel;
                        }
                        Goto::QueryList {
                            conn_id,
                            db_name,
                            kind,
                        } => {
                            self.query_list.set_data(&conn_id, &db_name, kind)?;
                            self.main_panel = MainPanel::QueryList;
                            self.focus = Focus::MainPanel;
                        }
                        Goto::ViewListMySQL { conn_id, db_name } => {
                            self.view_list_mysql.set_data(&conn_id, &db_name).await?;
                            self.main_panel = MainPanel::ViewListMySQL;
                            self.focus = Focus::MainPanel;
                        }
                        Goto::ViewListPG {
                            conn_id,
                            db_name,
                            schema_name,
                        } => {
                            self.view_list_pg
                                .set_data(&conn_id, &db_name, &schema_name)
                                .await?;
                            self.main_panel = MainPanel::ViewListPG;
                            self.focus = Focus::MainPanel;
                        }
                        Goto::UserListMySQL { conn_id } => {
                            self.user_list_mysql.set_data(&conn_id).await?;
                            self.main_panel = MainPanel::UserListMySQL;
                            self.focus = Focus::MainPanel;
                        }
                        Goto::RoleListPG { conn_id } => {
                            self.role_list_pg.set_data(&conn_id).await?;
                            self.main_panel = MainPanel::RoleListPG;
                            self.focus = Focus::MainPanel;
                        }
                        _ => (),
                    },
                    ComponentResult::Focus(focus) => {
                        self.focus = focus;
                    }
                    _ => (),
                };
            }
            Focus::MainPanel => {
                match self.main_panel {
                    MainPanel::Home => {
                        if let ComponentResult::Focus(focus) = self.home.handle_event(key) {
                            self.focus = focus;
                        }
                    }
                    MainPanel::TableListMySQL => {
                        match self.table_list_mysql.handle_event(key).await? {
                            ComponentResult::Goto(goto) => match goto {
                                Goto::TableDetailMySQL {
                                    conn_id,
                                    db_name,
                                    table_name,
                                } => {
                                    self.table_detail_mysql
                                        .set_data(&conn_id, &db_name, table_name.as_deref())
                                        .await?;
                                    self.main_panel = MainPanel::TableDetailMySQL;
                                }
                                Goto::DataListMySQL {
                                    conn_id,
                                    db_name,
                                    table_name,
                                } => {
                                    self.data_list_mysql
                                        .set_data(
                                            &conn_id,
                                            &db_name,
                                            &table_name,
                                            MainPanel::TableListMySQL,
                                        )
                                        .await?;
                                    self.main_panel = MainPanel::DataListMySQL;
                                }
                                _ => (),
                            },
                            ComponentResult::Focus(focus) => {
                                self.focus = focus;
                            }
                            _ => (),
                        }
                    }
                    MainPanel::TableListPG => match self.table_list_pg.handle_event(key).await? {
                        ComponentResult::Goto(goto) => match goto {
                            Goto::TableDetailPG {
                                conn_id,
                                db_name,
                                schema_name,
                                table_name,
                            } => {
                                self.table_detail_pg
                                    .set_data(
                                        &conn_id,
                                        &db_name,
                                        &schema_name,
                                        table_name.as_deref(),
                                    )
                                    .await?;
                                self.main_panel = MainPanel::TableDetailPG;
                            }
                            Goto::DataListPG {
                                conn_id,
                                db_name,
                                schema_name,
                                table_name,
                            } => {
                                self.data_list_pg
                                    .set_data(
                                        &conn_id,
                                        &db_name,
                                        &schema_name,
                                        &table_name,
                                        MainPanel::TableListPG,
                                    )
                                    .await?;
                                self.main_panel = MainPanel::DataListPG;
                            }
                            _ => (),
                        },
                        ComponentResult::Focus(focus) => {
                            self.focus = focus;
                        }
                        _ => (),
                    },
                    MainPanel::TableDetailMySQL => {
                        if let ComponentResult::Back(_) =
                            self.table_detail_mysql.handle_event(key).await?
                        {
                            self.main_panel = MainPanel::TableListMySQL;
                        }
                    }
                    MainPanel::TableDetailPG => {
                        match self.table_detail_pg.handle_event(key).await? {
                            ComponentResult::Back(_) => {
                                self.main_panel = MainPanel::TableListPG;
                            }
                            ComponentResult::BackRefresh(_) => {
                                self.main_panel = MainPanel::TableListPG;
                                self.table_list_pg.refresh().await?;
                            }
                            _ => (),
                        }
                    }
                    MainPanel::DataListMySQL => {
                        if let ComponentResult::Back(panel) =
                            self.data_list_mysql.handle_event(key).await?
                        {
                            self.main_panel = panel;
                        }
                    }
                    MainPanel::DataListPG => {
                        if let ComponentResult::Back(panel) =
                            self.data_list_pg.handle_event(key).await?
                        {
                            self.main_panel = panel;
                        }
                    }
                    MainPanel::QueryList => match self.query_list.handle_event(key)? {
                        ComponentResult::Goto(goto) => match goto {
                            Goto::QueryDetailMySQL {
                                conn_id,
                                db_name,
                                query_name,
                            } => {
                                self.query_detail_mysql.set_data(
                                    &conn_id,
                                    &db_name,
                                    query_name.as_deref(),
                                )?;
                                self.main_panel = MainPanel::QueryDetailMySQL;
                            }
                            Goto::QueryDetailPG {
                                conn_id,
                                db_name,
                                query_name,
                            } => {
                                self.query_detail_pg.set_data(
                                    &conn_id,
                                    &db_name,
                                    query_name.as_deref(),
                                )?;
                                self.main_panel = MainPanel::QueryDetailPG;
                            }
                            _ => (),
                        },
                        ComponentResult::Focus(focus) => self.focus = focus,
                        _ => (),
                    },
                    MainPanel::QueryDetailMySQL => {
                        match self.query_detail_mysql.handle_event(key).await? {
                            ComponentResult::Back(_) => self.main_panel = MainPanel::QueryList,
                            ComponentResult::Focus(focus) => self.focus = focus,
                            _ => (),
                        }
                    }
                    MainPanel::QueryDetailPG => {
                        match self.query_detail_pg.handle_event(key).await? {
                            ComponentResult::Back(_) => self.main_panel = MainPanel::QueryList,
                            ComponentResult::Focus(focus) => self.focus = focus,
                            _ => (),
                        }
                    }
                    MainPanel::ViewListMySQL => {
                        match self.view_list_mysql.handle_event(key).await? {
                            ComponentResult::Goto(goto) => match goto {
                                Goto::ViewDetailMySQL {
                                    conn_id,
                                    db_name,
                                    view_name,
                                } => {
                                    self.view_detail_mysql
                                        .set_data(&conn_id, &db_name, view_name.as_deref())
                                        .await?;
                                    self.main_panel = MainPanel::ViewDetailMySQL;
                                    self.focus = Focus::MainPanel;
                                }
                                Goto::DataListMySQL {
                                    conn_id,
                                    db_name,
                                    table_name,
                                } => {
                                    self.data_list_mysql
                                        .set_data(
                                            &conn_id,
                                            &db_name,
                                            &table_name,
                                            MainPanel::ViewListMySQL,
                                        )
                                        .await?;
                                    self.main_panel = MainPanel::DataListMySQL;
                                }
                                _ => (),
                            },

                            ComponentResult::Focus(focus) => self.focus = focus,
                            _ => (),
                        }
                    }
                    MainPanel::ViewDetailMySQL => {
                        match self.view_detail_mysql.handle_event(key).await? {
                            ComponentResult::Back(_) => self.main_panel = MainPanel::ViewListMySQL,
                            ComponentResult::Focus(focus) => self.focus = focus,
                            _ => (),
                        }
                    }
                    MainPanel::ViewListPG => match self.view_list_pg.handle_event(key).await? {
                        ComponentResult::Goto(goto) => match goto {
                            Goto::ViewDetailPG {
                                conn_id,
                                db_name,
                                schema_name,
                                view_name,
                            } => {
                                self.view_detail_pg
                                    .set_data(
                                        &conn_id,
                                        &db_name,
                                        &schema_name,
                                        view_name.as_deref(),
                                    )
                                    .await?;
                                self.main_panel = MainPanel::ViewDetailPG;
                                self.focus = Focus::MainPanel;
                            }
                            Goto::DataListPG {
                                conn_id,
                                db_name,
                                schema_name,
                                table_name,
                            } => {
                                self.data_list_pg
                                    .set_data(
                                        &conn_id,
                                        &db_name,
                                        &schema_name,
                                        &table_name,
                                        MainPanel::ViewListPG,
                                    )
                                    .await?;
                                self.main_panel = MainPanel::DataListPG;
                                self.focus = Focus::MainPanel;
                            }

                            _ => (),
                        },
                        ComponentResult::Focus(focus) => self.focus = focus,
                        _ => (),
                    },
                    MainPanel::ViewDetailPG => match self.view_detail_pg.handle_event(key).await? {
                        ComponentResult::Back(_) => self.main_panel = MainPanel::ViewListPG,
                        ComponentResult::Focus(focus) => self.focus = focus,
                        _ => (),
                    },
                    MainPanel::UserListMySQL => {
                        match self.user_list_mysql.handle_event(key).await? {
                            ComponentResult::Goto(Goto::UserDetailMySQL {
                                conn_id,
                                user_host,
                                user_name,
                            }) => {
                                self.user_detail_mysql
                                    .set_data(&conn_id, user_host.as_deref(), user_name.as_deref())
                                    .await?;
                                self.main_panel = MainPanel::UserDetailMySQL;
                                self.focus = Focus::MainPanel;
                            }
                            ComponentResult::Focus(focus) => self.focus = focus,
                            _ => (),
                        }
                    }
                    MainPanel::UserDetailMySQL => {
                        match self.user_detail_mysql.handle_event(key).await? {
                            ComponentResult::Back(_) => self.main_panel = MainPanel::UserListMySQL,
                            ComponentResult::Focus(focus) => self.focus = focus,
                            _ => (),
                        }
                    }
                    MainPanel::RoleListPG => match self.role_list_pg.handle_event(key).await? {
                        ComponentResult::Goto(Goto::RoleDetailPG { conn_id, role_name }) => {
                            self.role_detail_pg
                                .set_data(&conn_id, role_name.as_deref())
                                .await?;
                            self.main_panel = MainPanel::RoleDetailPG;
                            self.focus = Focus::MainPanel;
                        }
                        ComponentResult::Focus(focus) => self.focus = focus,
                        _ => (),
                    },
                    MainPanel::RoleDetailPG => match self.role_detail_pg.handle_event(key).await? {
                        ComponentResult::Back(_) => self.main_panel = MainPanel::RoleListPG,
                        ComponentResult::Focus(focus) => self.focus = focus,
                        _ => (),
                    },
                };
            }
        }
        Ok(())
    }
    pub fn draw_component<B>(&mut self, f: &mut Frame<B>) -> Result<()>
    where
        B: Backend,
    {
        let fsize = f.size();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(2)].as_ref())
            .split(fsize);

        self.command_bar.borrow().draw(f, chunks[0]);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 4), Constraint::Ratio(3, 4)])
            .split(chunks[1]);

        self.connection_list
            .draw(f, chunks[0], self.focus == Focus::LeftPanel);

        match self.main_panel {
            MainPanel::Home => {
                self.home.draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::QueryList => {
                self.query_list
                    .draw(f, chunks[1], self.focus == Focus::MainPanel)
            }
            MainPanel::QueryDetailMySQL => {
                self.query_detail_mysql
                    .draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::QueryDetailPG => {
                self.query_detail_pg
                    .draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::TableListMySQL => {
                self.table_list_mysql
                    .draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::DataListMySQL => {
                self.data_list_mysql
                    .draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::TableDetailMySQL => {
                self.table_detail_mysql
                    .draw(f, chunks[1], self.focus == Focus::MainPanel)?;
            }
            MainPanel::TableListPG => {
                self.table_list_pg
                    .draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::DataListPG => {
                self.data_list_pg
                    .draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::TableDetailPG => {
                self.table_detail_pg
                    .draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::ViewListMySQL => {
                self.view_list_mysql
                    .draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::ViewDetailMySQL => {
                self.view_detail_mysql
                    .draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::ViewListPG => {
                self.view_list_pg
                    .draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::ViewDetailPG => {
                self.view_detail_pg
                    .draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::UserListMySQL => {
                self.user_list_mysql
                    .draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::UserDetailMySQL => {
                self.user_detail_mysql
                    .draw(f, chunks[1], self.focus == Focus::MainPanel)?;
            }
            MainPanel::RoleListPG => {
                self.role_list_pg
                    .draw(f, chunks[1], self.focus == Focus::MainPanel);
            }
            MainPanel::RoleDetailPG => {
                self.role_detail_pg
                    .draw(f, chunks[1], self.focus == Focus::MainPanel)?;
            }
        }
        Ok(())
    }
    pub fn draw_dialog<B>(&mut self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        match self.focus {
            Focus::LeftPanel => {
                self.connection_list.draw_dialog(f);
            }
            Focus::MainPanel => match self.main_panel {
                MainPanel::QueryList => self.query_list.draw_dialog(f),
                MainPanel::QueryDetailMySQL => self.query_detail_mysql.draw_dialog(f),
                MainPanel::QueryDetailPG => self.query_detail_pg.draw_dialog(f),
                MainPanel::TableListMySQL => self.table_list_mysql.draw_dialog(f),
                MainPanel::DataListMySQL => self.data_list_mysql.draw_dialog(f),
                MainPanel::TableDetailMySQL => self.table_detail_mysql.draw_dialog(f),
                MainPanel::TableListPG => self.table_list_pg.draw_dialog(f),
                MainPanel::DataListPG => self.data_list_pg.draw_dialog(f),
                MainPanel::TableDetailPG => self.table_detail_pg.draw_dialog(f),
                MainPanel::ViewListMySQL => self.view_list_mysql.draw_dialog(f),
                MainPanel::ViewDetailMySQL => self.view_detail_mysql.draw_dialog(f),
                MainPanel::ViewListPG => self.view_list_pg.draw_dialog(f),
                MainPanel::ViewDetailPG => self.view_detail_pg.draw_dialog(f),
                MainPanel::UserListMySQL => self.user_list_mysql.draw_dialog(f),
                MainPanel::UserDetailMySQL => self.user_detail_mysql.draw_dialog(f),
                MainPanel::RoleListPG => self.role_list_pg.draw_dialog(f),
                MainPanel::RoleDetailPG => self.role_detail_pg.draw_dialog(f),
                _ => (),
            },
        }
        if let Some(dlg) = &self.error_dlg {
            dlg.draw(f);
        }
    }
}
