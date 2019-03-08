extern crate env_logger;
extern crate failure;
extern crate libc;
extern crate terminal_size;
extern crate termion;
extern crate unicode_width;

use std::fs::File;
use std::io::{self, BufRead, BufReader, Stdout};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};

use failure::Error;
use termion::raw::{IntoRawMode, RawTerminal};

mod event;
mod ui;

use self::event::Event;
use self::ui::{Input, Prompt};

pub type CommandWithArgument = (String, Vec<String>);

pub struct ManWith {
    source: Arc<Mutex<Option<BufReader<File>>>>,
    prompt: Arc<Mutex<Prompt<RawTerminal<Stdout>>>>,
}

impl ManWith {
    pub fn new(cmd: &str, height: usize) -> Self {
        let stdout = io::stdout();
        let source = source();
        let stdout = stdout.into_raw_mode().unwrap();
        let prompt = Arc::new(Mutex::new(Prompt::new(stdout, cmd, height)));

        ManWith {
            source: Arc::new(Mutex::new(source)),
            prompt: prompt,
        }
    }

    pub fn run(&self) -> Result<CommandWithArgument, Error> {
        {
            let mut p = self.prompt.lock().unwrap();
            p.show()?;
            p.flush()?;
        }

        let (tx, rx) = mpsc::channel();
        let th = {
            self.input_handler(tx.clone());
            self.event_handler(tx.clone(), rx)
        };

        thread::spawn(move || {
            let _ = Input::reader(tx.clone());
        });

        let _ = th.join();

        let mut f = self.prompt.lock().unwrap();
        ui::cursor::holizon(&mut f.stdout, 1u64);
        ui::cursor::clear_line(&mut f.stdout);
        f.flush()?;

        Ok((f.command.clone(), f.argument.clone()))
    }

    pub fn input_handler(&self, tx: Sender<Event>) -> JoinHandle<()> {
        let source = self.source.clone();

        thread::spawn(move || loop {
            let mut src = source.lock().unwrap();

            if let Some(ref mut b) = *src {
                let mut buf = vec![];

                match b.read_until(b'\n', &mut buf) {
                    Ok(n) if n != 0 => {
                        if buf.ends_with(&[b'\n']) || buf.ends_with(&[b'\0']) {
                            buf.pop();
                        }
                        let l = String::from_utf8(buf).unwrap_or(String::new());
                        let _ = tx.send(Event::ReadLine(l));
                    }
                    _ => {}
                }
            }
        })
    }

    pub fn event_handler(&self, _tx: Sender<Event>, rx: Receiver<Event>) -> JoinHandle<()> {
        let prompt = self.prompt.clone();

        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(Event::Quit) => {
                        // Quit message.
                        break;
                    }
                    Ok(Event::ReadLine(line)) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.panel.push(line);
                            f.show().unwrap();
                            f.flush().unwrap();
                            Ok(())
                        });
                    }
                    Ok(Event::Key(ch)) => {
                        // From keyboad.
                        let _ = prompt.lock().and_then(|mut f| {
                            f.insert(ch);
                            Ok(())
                        });
                    }
                    Ok(Event::Backspace) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.backspace();
                            Ok(())
                        });
                    }
                    Ok(Event::Enter) => {
                        let mut f = prompt.lock().unwrap();

                        if f.input.len() > 0 {
                            f.append(); // Append command arguments.
                        } else {
                            break;
                        }
                    }
                    Ok(Event::Up) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.up();
                            Ok(())
                        });
                    }
                    Ok(Event::Down) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.down();
                            Ok(())
                        });
                    }
                    Ok(Event::Next) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.next();
                            Ok(())
                        });
                    }
                    Ok(Event::Prev) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.prev();
                            Ok(())
                        });
                    }
                    _ => break,
                };

                let _ = prompt.lock().and_then(|mut f| {
                    f.show().and_then(|_| f.flush()).unwrap();
                    Ok(())
                });
            }
        })
    }
}

fn source() -> Option<BufReader<File>> {
    unsafe {
        let isatty = libc::isatty(libc::STDIN_FILENO as i32) != 0;

        if !isatty {
            let stdin = File::from_raw_fd(libc::dup(libc::STDIN_FILENO));
            let file = File::open("/dev/tty").unwrap();
            libc::dup2(file.into_raw_fd(), libc::STDIN_FILENO);

            return Some(BufReader::new(stdin));
        }
    }

    None
}
