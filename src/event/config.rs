use crate::event::{Key, KeyCode as Code, KeyModifier as Mod};

pub const NEW_KEY: Key = Key {
    code: Code::Char('n'),
    modifier: Mod::Ctrl,
};
pub const EDIT_KEY: Key = Key {
    code: Code::Char('e'),
    modifier: Mod::Ctrl,
};
pub const DELETE_KEY: Key = Key {
    code: Code::Char('d'),
    modifier: Mod::Ctrl,
};
pub const REFRESH_KEY: Key = Key {
    code: Code::Char('r'),
    modifier: Mod::Ctrl,
};
pub const UP_KEY: Key = Key {
    code: Code::Up,
    modifier: Mod::None,
};
pub const DOWN_KEY: Key = Key {
    code: Code::Down,
    modifier: Mod::None,
};
pub const LEFT_KEY: Key = Key {
    code: Code::Left,
    modifier: Mod::None,
};
pub const RIGHT_KEY: Key = Key {
    code: Code::Right,
    modifier: Mod::None,
};
pub const BACK_KEY: Key = Key {
    code: Code::Esc,
    modifier: Mod::None,
};
pub const CANCEL_KEY: Key = Key {
    code: Code::Esc,
    modifier: Mod::None,
};
pub const CONFIRM_KEY: Key = Key {
    code: Code::Enter,
    modifier: Mod::None,
};
pub const SAVE_KEY: Key = Key {
    code: Code::Char('s'),
    modifier: Mod::Ctrl,
};
pub const SWITCH_KEY: Key = Key {
    code: Code::Tab,
    modifier: Mod::None,
};
pub const RUN_KEY: Key = Key {
    code: Code::Char('r'),
    modifier: Mod::Ctrl,
};
pub const CLEAR_KEY: Key = Key {
    code: Code::Backspace,
    modifier: Mod::None,
};
pub const TAB_LEFT_KEY: Key = if cfg!(windows) {
    Key {
        code: Code::Tab,
        modifier: Mod::Shift,
    }
} else {
    Key {
        code: Code::Tab,
        modifier: Mod::Alt,
    }
};
pub const TAB_RIGHT_KEY: Key = {
    Key {
        code: Code::Tab,
        modifier: Mod::None,
    }
};
pub const PAGE_NEXT_KEY: Key = Key {
    code: Code::Char('f'),
    modifier: Mod::Ctrl,
};
pub const PAGE_PRIV_KEY: Key = Key {
    code: Code::Char('y'),
    modifier: Mod::Ctrl,
};
pub const MOVE_UP_KEY: Key = Key {
    code: Code::Up,
    modifier: Mod::Shift,
};
pub const MOVE_DOWN_KEY: Key = Key {
    code: Code::Down,
    modifier: Mod::Shift,
};
pub const USER_KEY: Key = Key {
    code: Code::Char('u'),
    modifier: Mod::Ctrl,
};
pub const QUIT_APP_KEY: Key = Key {
    code: Code::Char('c'),
    modifier: Mod::Ctrl,
};
pub const SPACE_KEY: Key = Key {
    code: Code::Char(' '),
    modifier: Mod::None,
};
pub const SET_NULL_KEY: Key = Key {
    code: Code::Char('n'),
    modifier: Mod::Ctrl,
};
