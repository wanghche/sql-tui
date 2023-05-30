mod command_bar;
mod connection_list;
mod home;
mod mysql;
mod pg;
mod query_list;

pub fn get_table_up_index(index: Option<usize>) -> usize {
    if let Some(i) = index {
        if i > 0 {
            i - 1
        } else {
            0
        }
    } else {
        0
    }
}
pub fn get_table_down_index(index: Option<usize>, len: usize) -> usize {
    if let Some(i) = index {
        if i < len - 1 {
            i + 1
        } else {
            len - 1
        }
    } else {
        0
    }
}

pub use self::{command_bar::*, connection_list::*, home::*, mysql::*, pg::*, query_list::*};
