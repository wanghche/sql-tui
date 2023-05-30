use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fmt::{Display, Formatter, Result};
use tui_textarea::{Input, Key as TextAreaKey};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Code {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Backspace,
    Tab,
    Char(char),
    F(u8),
    Unknown,
}
impl Display for Code {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Code::Up => write!(f, "\u{2191}"),
            Code::Down => write!(f, "\u{2193}"),
            Code::Enter => write!(f, "\u{23ce}"),
            Code::Left => write!(f, "\u{2190}"),
            Code::Right => write!(f, "\u{2192}"),
            Code::Esc => write!(f, "Esc"),
            Code::Backspace => write!(f, "\u{232b}"),
            Code::Tab => write!(f, "Tab"),
            Code::Char(c) => write!(f, "{}", c),
            Code::F(n) => write!(f, "F{}", n),
            Code::Unknown => write!(f, ""),
        }
    }
}
impl From<KeyCode> for Code {
    fn from(key_code: KeyCode) -> Self {
        match key_code {
            KeyCode::Up => Code::Up,
            KeyCode::Down => Code::Down,
            KeyCode::Left => Code::Left,
            KeyCode::Right => Code::Right,
            KeyCode::Enter => Code::Enter,
            KeyCode::Esc => Code::Esc,
            KeyCode::Backspace => Code::Backspace,
            KeyCode::Tab => Code::Tab,
            KeyCode::Char(c) => Code::Char(c),
            KeyCode::F(n) => Code::F(n),
            _ => Code::Unknown,
        }
    }
}
impl Into<TextAreaKey> for Code {
    fn into(self) -> TextAreaKey {
        match self {
            Code::Up => TextAreaKey::Up,
            Code::Down => TextAreaKey::Down,
            Code::Left => TextAreaKey::Left,
            Code::Right => TextAreaKey::Right,
            Code::Enter => TextAreaKey::Enter,
            Code::Esc => TextAreaKey::Esc,
            Code::Backspace => TextAreaKey::Backspace,
            Code::Tab => TextAreaKey::Tab,
            Code::Char(c) => TextAreaKey::Char(c),
            Code::F(n) => TextAreaKey::F(n),
            Code::Unknown => TextAreaKey::Null,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Modifier {
    Ctrl,
    Shift,
    Alt,
    None,
}
impl From<KeyModifiers> for Modifier {
    fn from(key_modifiers: KeyModifiers) -> Self {
        match key_modifiers {
            KeyModifiers::SHIFT => Modifier::Shift,
            KeyModifiers::CONTROL => Modifier::Ctrl,
            KeyModifiers::ALT => Modifier::Alt,
            _ => Modifier::None,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Key {
    pub code: Code,
    pub modifier: Modifier,
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self.modifier {
            Modifier::Ctrl => write!(f, "Ctrl+{}", self.code.to_string().to_uppercase()),
            Modifier::Shift => write!(f, "{}", self.code.to_string().to_uppercase()),
            Modifier::Alt => write!(f, "Alt+{}", self.code),
            Modifier::None => write!(f, "{}", self.code),
        }
    }
}

impl From<KeyEvent> for Key {
    fn from(key_event: KeyEvent) -> Self {
        Key {
            code: Code::from(key_event.code),
            modifier: Modifier::from(key_event.modifiers),
        }
    }
}

impl Into<Input> for Key {
    fn into(self) -> Input {
        Input {
            key: self.code.into(),
            ctrl: matches!(self.modifier, Modifier::Ctrl),
            alt: matches!(self.modifier, Modifier::Alt),
        }
    }
}
