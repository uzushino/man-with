extern crate env_logger;
extern crate failure;
extern crate libc;
extern crate shellexpand;
extern crate terminal_size;
extern crate termion;
extern crate unicode_width;

use std::fs::File;
use std::io::{self, BufRead, BufReader, Stdout};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::path::PathBuf;
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};

use failure::Error;
use termion::raw::{IntoRawMode, RawTerminal};

mod ui;
mod event;

use self::event::Event;
use self::ui::{
    viewer::ShowType, 
    Input, 
    prompt::{
        Prompt,
        PromptMode,
    }
};

pub type CommandWithArgument = (String, Vec<String>);

pub struct ManWith {
    source: Arc<Mutex<Option<BufReader<File>>>>,
    prompt: Arc<Mutex<Prompt<RawTerminal<Stdout>>>>,
}

impl ManWith {
    pub fn new(cmd: &str, height: usize, help: bool, history_path: Option<PathBuf>) -> Self {
        let stdout = io::stdout();
        let source = source();
        let stdout = stdout.into_raw_mode().unwrap();
        let prompt = Arc::new(Mutex::new(Prompt::new(
            stdout,
            cmd,
            height,
            help,
            source.is_some(),
            history_path,
        )));

        ManWith {
            source: Arc::new(Mutex::new(source)),
            prompt: prompt,
        }
    }

    pub fn run(&self) -> Result<CommandWithArgument, Error> {
        {
            let mut f = self.prompt.lock().unwrap();

            f.read_history();
            f.show()?;
            f.flush()?;
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

        ui::cursor::horizon(&mut f.stdout, 1u64);
        ui::cursor::clear_line(&mut f.stdout);
        
        f.write_history();
        f.flush()?;

        Ok(f.full_command())
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
                        let _ = prompt.lock().and_then(|mut f| {
                            f.quit();
                            Ok(())
                        });
                        break;
                    }
                    Ok(Event::ReadLine(line)) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.insert_line(line);
                            Ok(())
                        });
                    }
                    Ok(Event::Key(ch)) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            match f.get_mode() {
                                ui::prompt::PromptMode::Choose => {},
                                ui::prompt::PromptMode::Prompt | ui::prompt::PromptMode::File => {
                                    match ch {
                                        ' ' => f.append(),
                                        _ => f.insert(ch)
                                    }
                                },
                                _ => {}
                            }

                            Ok(())
                        });
                    }
                    Ok(Event::Candidate(candidate)) if candidate == PromptMode::File => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.set_mode(ui::prompt::PromptMode::File);
                            Ok(())
                        });
                    }
                    Ok(Event::Candidate(candidate)) if candidate == PromptMode::Choose => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.set_mode(ui::prompt::PromptMode::Choose);
                            Ok(())
                        });
                    }
                    Ok(Event::Backspace) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.backspace();
                            Ok(())
                        });
                    }
                    Ok(Event::Delete) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.delete();
                            Ok(())
                        });
                    }
                    Ok(Event::Tab) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.completion();
                            Ok(())
                        });
                    }
                    Ok(Event::Enter) => {
                        let mut f = prompt.lock().unwrap();
                        match f.get_mode() {
                            ui::prompt::PromptMode::Choose => {
                                let a = f.current_input();
                                match a.as_ref() {
                                    "man" => {
                                        f.set_mode(PromptMode::Prompt)
                                    },
                                    "file" => {
                                        f.set_mode(PromptMode::File)
                                    },
                                    _ => {}
                                }
                            },
                            ui::prompt::PromptMode::Prompt => {
                                if f.cursor > 0 {
                                    if !f.argument.is_empty() {
                                        f.append();
                                    }
                                } else if f.is_last() {
                                    break;
                                }
                            },
                            ui::prompt::PromptMode::File => {
                                let line = f.current_buffer_line().clone();
                                f.append_argument(line);
                                f.append();
                            },
                            _ => f.append(),
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
                    Ok(Event::Left) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.select_back();
                            Ok(())
                        });
                    }
                    Ok(Event::Right) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.select_forward();
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
                    Ok(Event::Forward) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.cursor_forward();
                            Ok(())
                        });
                    }
                    Ok(Event::Back) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.cursor_back();
                            Ok(())
                        });
                    }
                    Ok(Event::Fn1) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.viewer.toggle_show_type(ShowType::LineNumber);
                            Ok(())
                        });
                    }
                    Ok(Event::Fn2) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.incr_size();
                            Ok(())
                        });
                    }
                    Ok(Event::Fn3) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.decr_size();
                            Ok(())
                        });
                    }
                    Ok(Event::History) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.set_mode(ui::prompt::PromptMode::History);
                            f.history_back();

                            Ok(())
                        });
                    }
                    Ok(Event::MoveTo(0)) => {
                        let _ = prompt.lock().and_then(|mut f| {
                            f.beginning_of_line();
                            Ok(())
                        });
                    }
                    Ok(Event::MoveTo(n)) => {
                        if n < 0 {
                            let _ = prompt.lock().and_then(|mut f| {
                                f.end_of_line();
                                Ok(())
                            });
                        }
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
