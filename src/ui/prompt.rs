use std::io::{ Write, BufRead };
use std::path::PathBuf;
use serde_derive::{ Serialize, Deserialize };

use terminal_size::terminal_size;
use termion;
use super::viewer::{SourceType, Viewer};
use crate::ui::cursor;

const PROMPT: &'static str = "> ";

#[derive(Serialize, Deserialize)]
struct History {
    command: String, 
    argument: Vec<String>,
}

#[derive(Clone, PartialEq)]
pub enum PromptMode {
    Prompt,
    History,
    File,
    Choose,
}

impl History {
    fn write(&self, history: &PathBuf) {
        let json = serde_json::to_string(self).unwrap();

        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(history)
            .and_then(|mut f| {
                f.write_all(json.as_bytes())
                .and_then(move |_| f.write(b"\n"))
            })
            .unwrap();
    }
    
    fn read(command: &String, history: &PathBuf) -> Vec<Vec<String>> {
        if !std::path::Path::new(history).exists() {
            return Vec::default()
        }

        let f = std::fs::File::open(history);
        let mut arguments: Vec<Vec<String>> = Vec::default();

        if let Ok(file) = f {
            let lines = std::io::BufReader::new(file).lines();
            for line in lines {
                let l = line.unwrap();
                let hist: Result<History, _> = serde_json::from_str(l.as_str());

                match hist {
                    Ok(mut h) => {
                        if *command == h.command {
                            h.argument.push(String::default());
                            arguments.push(h.argument.clone())
                        }
                    }
                    _ =>  {}
                }
            }
        }

        arguments
    }
}

#[derive(Clone)]
pub struct Prompt<T: Write + Send + Drop> {
    panel: Vec<String>,
    pub command: String,
    pub stdout: T,
    pub cursor: usize,
    pub viewer: Viewer,
    pub mode: PromptMode,
    pub argument: Vec<String>,
    pub completion: Option<String>,
    buffer: Vec<String>,
    pos: usize,
    size: usize,
    selected: usize,
    history_index: u64,
    history_path: Option<PathBuf>,
    histories: Vec<Vec<String>>,
}

fn is_args(ch: char) -> bool {
    match ch {
        '-' | '_' | '=' | ':' | '{' | '}' | '.' => true,
        _ => ch.is_ascii_alphabetic() || ch.is_ascii_digit(),
    }
}

impl<T: Write + Send + Drop> Prompt<T> {
    pub fn new(stdout: T, command: &str, height: usize, help: bool, stdin: bool, history_path: Option<PathBuf>) -> Self {
        let viewer = match (stdin, help) {
            (true, _) => Viewer::new(command, SourceType::Stdin),
            (_, true) => Viewer::new(command, SourceType::Help),
            _ => Viewer::new(command, SourceType::Man),
        };
       
        let buffer = { 
            viewer.source() 
        };

        Prompt {
            panel: vec![String::new(); height],
            command: String::from(command),
            argument: vec![String::default()],
            stdout,
            completion: None,
            viewer: viewer,
            buffer: buffer
                .split('\n')
                .map(ToString::to_string)
                .collect::<Vec<String>>(),
            cursor: 0,
            pos: 0,
            size: height,
            selected: 0,
            history_index: 0u64,
            history_path,
            histories: Vec::default(),
            mode: PromptMode::Prompt,
        }
    }

    pub fn set_mode(&mut self, mode: PromptMode) {
        self.mode = mode;

        match self.mode {
            PromptMode::File => {
                self.buffer = self.viewer.file_path(None)
                    .split('\n')
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
            },
            PromptMode::Prompt => {
                self.buffer = self.viewer.source()
                    .split('\n')
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
            },
            PromptMode::Choose => {
                self.buffer = vec![
                    "man".to_owned(),
                    "file".to_owned(),
                ];
            }
            _ => { }
        }
    }

    pub fn get_mode<'a>(&'a self) -> &'a PromptMode {
        &self.mode
    }

    pub fn quit(&self) { 
    }

    pub fn write_history(&self) {
        if let Some(history) = &self.history_path {
            let args = self.argument
                .clone()
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            if !args.is_empty() {
                let hist = History {
                    command: self.command.clone(),
                    argument: args.clone(),
                };
                hist.write(history);
            }
        }
    }
    
    pub fn read_history(&mut self) {
        if let Some(history) = &self.history_path {
            let histories = History::read(&self.command, history);
            self.histories = histories;
        }
    }

    pub fn full_command(&self) -> (String, Vec<String>) {
        let a = self
            .argument
            .iter()
            .filter(|v| !v.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        (self.command.clone(), a.clone())
    }

    #[allow(dead_code)]
    pub fn change_command(&mut self, command: String) {
        self.command = command
    }

    pub fn down(&mut self) {
        if (self.pos + 1) > self.buffer.len() {
            self.pos = self.buffer.len();
        } else {
            self.pos += 1;
        }
    }

    pub fn up(&mut self) {
        let pos = self.pos as i64;

        if (pos - 1) < 0 {
            self.pos = 0;
        } else {
            self.pos -= 1;
        }
    }

    pub fn next(&mut self) {
        let s = self.pos + 1;
        let b = &self.buffer[s..self.buffer.len()];

        if let Some(n) = self.find_position(&b.to_vec()) {
            self.pos = s + n;
        }
    }

    pub fn prev(&mut self) {
        let e = self.pos - 1;
        let mut b = self.buffer[0..e].to_vec();
        b.reverse();

        if let Some(n) = self.find_position(&b) {
            self.pos = e - n - 1;
        }
    }

    pub fn select_back(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.cursor = 0;
            self.completion = None;
        }
    }

    pub fn select_forward(&mut self) {
        if self.selected < (self.argument.len() - 1) {
            self.selected += 1;
            self.cursor = 0;
            self.completion = None;
        }
    }
    
    pub fn end_of_line(&mut self) {
        if !self.argument.is_empty() {
            self.selected = self.argument.len() - 1;
            self.cursor = self.argument[self.selected].len();
        }
    }
    
    pub fn beginning_of_line(&mut self) {
        if !self.argument.is_empty() {
            self.selected = 0;
            self.cursor = 1;
        }
    }

    pub fn cursor_forward(&mut self) {
        let input = &self.argument[self.selected];

        if input.len() > self.cursor {
            self.cursor += 1;
        }
    }

    pub fn cursor_back(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.completion = None;
        }
    }

    fn viewpoint(&self) -> (usize, usize) {
        let (s, e) = if self.pos + self.size > self.buffer.len() {
            (
                (self.buffer.len() as isize) - (self.size as isize),
                self.buffer.len() as isize,
            )
        } else {
            (self.pos as isize, (self.pos + self.size) as isize)
        };
        let s = if s < 0 { 0 } else { s };

        (s as usize, e as usize)
    }

    pub fn backspace(&mut self) {
        let input = &mut self.argument[self.selected];

        if let Some(ch) = input[0..self.cursor].chars().rev().next() {
            self.cursor -= ch.len_utf8();
            input.remove(self.cursor);

            if let Some(n) = self.find_position(&self.buffer) {
                self.pos = n;
            }
        }
    }

    pub fn delete(&mut self) {
        let input = &mut self.argument[self.selected];

        if self.cursor == 0 || input[0..self.cursor].chars().next().is_some() {
            if input.len() > self.cursor {
                input.remove(self.cursor);
            }

            if let Some(n) = self.find_position(&self.buffer) {
                self.pos = n;
            }
        }
    }

    pub fn append(&mut self) {
        if self.is_last() {
            self.argument.push(String::default());
        }

        self.selected += 1;
        self.cursor = 0;
    }

    pub fn insert(&mut self, ch: char) {
        let input = &mut self.argument[self.selected];
        input.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();

        if let Some(n) = self.find_position(&self.buffer) {
            self.pos = n;
        }
    }

    pub fn insert_line(&mut self, line: String) {
        self.buffer.push(line)
    }

    fn candidates(&self) -> Vec<String> {
        if self.cursor == 0 {
            return Vec::default();
        }

        let n = &self.argument[self.selected];
        let hits = self
            .buffer
            .iter()
            .filter(|line| line.contains(n))
            .map(|line| line.split_whitespace().filter(|tok| tok.contains(n)))
            .flatten()
            .map(|token| token.matches(is_args).collect());

        let mut hits = hits.collect::<Vec<String>>();

        if n.starts_with(".") || n.starts_with("~") {
            // File path candidates
            let absolute = shellexpand::tilde(n);
            let absolute = PathBuf::from(absolute.as_ref());
            let dir = if absolute.is_dir() {
                absolute.as_path()
            } else {
                absolute.parent().unwrap()
            };

            let input_path = PathBuf::from(n);
            let input_path = if input_path.is_dir() {
                input_path.as_path()
            } else {
                input_path.parent().unwrap()
            };

            let paths = std::fs::read_dir(dir)
                .and_then(|p| Ok(p.into_iter().flatten().collect::<Vec<_>>()))
                .and_then(|paths| {
                    let dirs = paths
                        .iter()
                        .map(|p| {
                            p.path()
                                .file_name()
                                .map(|path| input_path.join(path).to_string_lossy().to_string())
                        })
                        .collect::<Vec<_>>();
                    Ok(dirs)
                });

            if let Ok(ps) = paths {
                hits.append(
                    &mut ps
                        .into_iter()
                        .flatten()
                        .filter(|p| p.starts_with(n))
                        .collect::<Vec<_>>(),
                )
            }
        }

        hits
    }

    pub fn completion(&mut self) {
        if let Some(comp) = &self.completion {
            let input = &mut self.argument[self.selected];
            input.push_str(&comp);

            self.cursor = input.len();
            self.completion = None;
            self.pos = 0;
        }
    }

    pub fn find_position(&self, buffer: &Vec<String>) -> Option<usize> {
        let input = &self.argument[self.selected];
        buffer.iter().position(|v| v.contains(input))
    }

    pub fn prompt() -> String {
        format!(
            "{}{}{}",
            termion::style::Bold,
            PROMPT,
            termion::style::Reset
        )
    }

    pub fn show_input(&mut self) -> Result<usize, std::io::Error> {
        let mut full_command = vec![self.command.clone()];
        full_command.extend(self.argument.clone());

        let p = format!(
            "{prompt}{bold}{white}{command}{reset}",
            prompt = Self::prompt(),
            bold = termion::style::Bold,
            white = termion::color::Fg(termion::color::White),
            reset = termion::style::Reset,
            command = full_command.join(" ")
        );

        self.stdout.write(p.as_bytes())
    }

    fn prompt_len(&mut self) -> u64 {
        let mut full_command = vec![self.command.clone()];
        let current = &self.argument[0..self.selected];
        full_command.extend(current.to_vec());

        PROMPT.len() as u64 + full_command.join(" ").len() as u64 + 1u64
    }

    pub fn incr_size(&mut self) {
        self.size += 1;
    }

    pub fn decr_size(&mut self) {
        self.size -= 1;
    }

    pub fn show_candidate(&mut self) -> Option<String> {
        if let Some(c) = self.candidates().first() {
            let input = &self.argument[self.selected];
            let comp = &c[input.len()..c.len()];

            return Some(comp.to_string());
        };
        None
    }

    pub fn show_viewer(&mut self) -> Vec<String> {
        let (s, e) = self.viewpoint();
        let mut buffer = self.viewer.show(self.buffer.clone());
        let input = &self.argument[self.selected];

        let decorated = format!(
            "{red}{input}{reset}",
            red = termion::color::Fg(termion::color::Red),
            input = input,
            reset = termion::style::Reset
        );
        buffer[self.pos] = buffer[self.pos].replace(input, &decorated);

        let lines = &buffer[s..e];
        for l in lines {
            self.stdout.write(l.as_bytes()).unwrap();

            cursor::down(&mut self.stdout, 1);
            cursor::horizon(&mut self.stdout, 1);
        }

        Vec::from(lines)
    }

    pub fn history_back(&mut self) {
        if let Some(_) = self.history_path {
            let hist = self.histories.iter().rev()
                .collect::<Vec<_>>();

            if let Some(hist) = hist.get(self.history_index as usize) {
                self.selected = (*hist).len() - 1;
                self.argument = (*hist).clone();

                self.cursor = 0;
                self.history_index += 1;
            }
        }
    }

    pub fn show(&mut self) -> Result<(), failure::Error> {
        self.sweep()?;

        let size = terminal_size();

        if let Some(_) = size {
            self.show_input()?;
            cursor::down(&mut self.stdout, 1);
            cursor::horizon(&mut self.stdout, 1 as u64);

            let lines = self.show_viewer();
            cursor::up(&mut self.stdout, lines.len() as u64);

            // Move cursor input position.
            cursor::up(&mut self.stdout, 1u64);

            let l = self.prompt_len();
            cursor::horizon(&mut self.stdout, l + self.cursor as u64 + 1);

            if let Some(comp) = self.show_candidate() {
                let s = format!(
                    "{color}{comp}{reset}",
                    color = termion::color::Fg(termion::color::Blue),
                    comp = comp,
                    reset = termion::style::Reset
                );

                cursor::horizon(&mut self.stdout, l + self.cursor as u64 + 1);
                self.stdout.write(s.as_bytes())?;
                self.completion = Some(comp);

                cursor::horizon(&mut self.stdout, l + self.cursor as u64 + 1);
            }
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), failure::Error> {
        self.stdout.flush()?;
        Ok(())
    }

    pub fn sweep(&mut self) -> Result<(), failure::Error> {
        // input
        cursor::horizon(&mut self.stdout, 1);
        cursor::clear_line(&mut self.stdout);
        self.stdout.write(b"\n")?;

        // panel
        for _ in 0..self.size {
            cursor::horizon(&mut self.stdout, 1);
            cursor::clear_line(&mut self.stdout);
            self.stdout.write(b"\n")?;
        }

        cursor::horizon(&mut self.stdout, 1);
        cursor::up(&mut self.stdout, (self.size + 1) as u64); // panel + input

        Ok(())
    }

    pub fn is_last(&self) -> bool {
        self.selected == self.argument.len() - 1
    }
}

mod test {
    use std::os::unix::io::{FromRawFd, IntoRawFd};
    use termion::raw::{IntoRawMode, RawTerminal};

    use super::*;

    #[test]
    fn end_of_line() {
        let stdout = std::io::stdout();
        let stdout = stdout.into_raw_mode().unwrap();
        let mut prompt = Prompt::new(
            stdout,
            &"diff".to_owned(),
            10,
            false,
            false,
            None,
        );

        prompt.argument.push("abc".to_string());
        prompt.end_of_line();

        assert_eq!(3, prompt.cursor)
    }
}