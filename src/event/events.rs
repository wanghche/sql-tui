use crate::event::key::{Code, Key, Modifier};
use crossterm::event;
use std::{sync::mpsc, thread, time::Duration};

pub struct EventConfig {
    pub exit_key: Key,
    pub tick_rate: Duration,
}

impl Default for EventConfig {
    fn default() -> EventConfig {
        EventConfig {
            exit_key: Key {
                code: Code::Char('c'),
                modifier: Modifier::Ctrl,
            },
            tick_rate: Duration::from_millis(250),
        }
    }
}

pub enum Event<I> {
    Input(I),
    Tick,
}

pub struct Events {
    rx: mpsc::Receiver<Event<Key>>,
    _tx: mpsc::Sender<Event<Key>>,
}

impl Events {
    pub fn new(tick_rate: u64) -> Events {
        Events::with_config(EventConfig {
            tick_rate: Duration::from_millis(tick_rate),
            ..Default::default()
        })
    }
    fn with_config(config: EventConfig) -> Events {
        let (tx, rx) = mpsc::channel();

        let event_tx = tx.clone();
        thread::spawn(move || loop {
            if event::poll(config.tick_rate).unwrap() {
                if let event::Event::Key(key) = event::read().unwrap() {
                    let key = Key::from(key);

                    event_tx.send(Event::Input(key)).unwrap();
                }
            }
            event_tx.send(Event::Tick).unwrap();
        });

        Events { _tx: tx, rx }
    }
    pub fn next(&self) -> Result<Event<Key>, mpsc::RecvError> {
        self.rx.recv()
    }
}
