use failure::Error;
use std::io::stdin;
use std::sync::mpsc::Sender;

use termion::event::Key;
use termion::input::TermRead;

use crate::event::Event;

pub struct Input {}

impl Input {
    pub fn reader(tx: Sender<Event>) -> Result<(), Error> {
        let stdin = stdin();

        for c in stdin.keys() {
            let _ = match c? {
                Key::Ctrl('a') => tx.send(Event::MoveTo(0))?,
                Key::Ctrl('e') => tx.send(Event::MoveTo(-1))?,
                Key::Ctrl('c') => {
                    tx.send(Event::Quit)?;
                    break;
                }
                Key::Ctrl('r') => tx.send(Event::History)?,
                Key::Ctrl('b') => tx.send(Event::Back)?,
                Key::Ctrl('f') => tx.send(Event::Forward)?,
                Key::Ctrl('p') => tx.send(Event::Prev)?,
                Key::Ctrl('n') => tx.send(Event::Next)?,
                Key::Char('\n') => tx.send(Event::Enter)?,
                Key::Char('\t') => tx.send(Event::Tab)?,
                Key::Char(c) => tx.send(Event::Key(c))?,
                Key::Backspace => tx.send(Event::Backspace)?,
                Key::Delete => tx.send(Event::Delete)?,
                Key::Left => tx.send(Event::Left)?,
                Key::Right => tx.send(Event::Right)?,
                Key::Down => tx.send(Event::Down)?,
                Key::Up => tx.send(Event::Up)?,
                Key::F(1) => tx.send(Event::Fn1)?,
                Key::F(2) => tx.send(Event::Fn2)?,
                Key::F(3) => tx.send(Event::Fn3)?,
                _ => {}
            };
        }

        Ok(())
    }
}
