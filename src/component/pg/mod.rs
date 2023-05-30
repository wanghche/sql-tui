mod data_list;
mod query_detail;
mod role_detail;
mod role_list;
mod table_detail;
mod table_list;
mod view_detail;
mod view_list;

pub use self::{
    data_list::DataListComponent as DataListComponentPG,
    query_detail::QueryDetailComponent as QueryDetailComponentPG,
    role_detail::RoleDetailComponent as RoleDetailComponentPG,
    role_list::RoleListComponent as RoleListComponentPG,
    table_detail::TableDetailComponent as TableDetailComponentPG,
    table_list::TableListComponent as TableListComponentPG,
    view_detail::ViewDetailComponent as ViewDetailComponentPG,
    view_list::ViewListComponent as ViewListComponentPG,
};
