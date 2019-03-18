use std::io::Write;
use std::process::Command;

use terminal_size::terminal_size;
use termion;

use crate::ui::cursor;

const PROMPT: &'static str = "> ";

#[derive(Clone)]
pub struct Prompt<T: Write + Send + Drop> {
    pub panel: Vec<String>,
    pub command: String,
    pub argument: Vec<String>,
    pub stdout: T,
    completation: Option<String>,
    buffer: Vec<String>,
    pub cursor: usize,
    pos: usize,
    size: usize,
    selected: usize,
}

fn is_args(ch: char) -> bool {
    match ch {
        '-' | '_' | '=' | ':' | '{' | '}' | '.' => true,
        _ => ch.is_ascii_alphabetic() || ch.is_ascii_digit()
    }
}

impl<T: Write + Send + Drop> Prompt<T> {
    pub fn new(stdout: T, command: &str, height: usize, help: bool, line_number: bool) -> Self {
        let cmd = if help {
            Command::new(command)
                .arg("--help")
                .output()
                .expect("failed to execute process")
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(format!("man {} | col -bx", command))
                .output()
                .expect("failed to execute process")
        };

        let out = cmd.stdout;
        let s = String::from_utf8_lossy(&out);
        let mut lines = Vec::default();
        for (i, line) in s.split('\n').collect::<Vec<_>>().iter().enumerate() {
            let l = if line_number {
                format!("{number} {line}", number = i + 1, line = line)
            } else {
                line.to_string()
            };
            lines.push(l);
        }

        Prompt {
            panel: vec![String::new(); height],
            command: String::from(command),
            argument: vec![String::default()],
            stdout: stdout,
            completation: None,
            buffer: lines,
            cursor: 0,
            pos: 0,
            size: height,
            selected: 0,
        }
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
           self.completation = None;
       } 
    }
    
    pub fn select_forward(&mut self) {
       if self.selected < (self.argument.len() - 1) {
           self.selected += 1;
           self.cursor = 0;
           self.completation = None;
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
            self.completation = None;
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

        if let Some(_) = input[0..self.cursor].chars().next() {
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

    fn candidates(&self) -> Vec<String> {
        if self.cursor == 0 {
            return Vec::default();
        }
        
        let n = &self.argument[self.selected];
        let hits = self.buffer.iter()
            .filter(|line| line.contains(n))
            .map(|line| line.split_whitespace().filter(|tok| tok.contains(n)))
            .flatten()
            .map(|token| token.matches(is_args).collect());
        
        hits.collect::<Vec<String>>()
    }

    pub fn completation(&mut self) {
        if let Some(comp) = &self.completation {
            let input = &mut self.argument[self.selected];
            input.push_str(&comp);

            self.cursor = input.len();
            self.completation = None;
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

        let p = format!("{prompt}{bold}{white}{command}{reset}", 
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
        let mut buffer = self.buffer.clone();
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
            cursor::holizon(&mut self.stdout, 1);
        }

        Vec::from(lines)
    }

    pub fn show(&mut self) -> Result<(), failure::Error> {
        self.sweep()?;

        let size = terminal_size();

        if let Some(_) = size {
            self.show_input()?;
            cursor::down(&mut self.stdout, 1);
            cursor::holizon(&mut self.stdout, 1 as u64);

            let lines = self.show_viewer();
            cursor::up(&mut self.stdout, lines.len() as u64);

            // Move cursor input position.
            cursor::up(&mut self.stdout, 1u64);

            let l = self.prompt_len();
            cursor::holizon(&mut self.stdout, l + self.cursor as u64 + 1);

            if let Some(comp) = self.show_candidate() {
                let s = format!(
                    "{color}{comp}{reset}",
                    color = termion::color::Fg(termion::color::Black),
                    comp = comp,
                    reset = termion::style::Reset
                );

                cursor::holizon(&mut self.stdout, l + self.cursor as u64 + 1);
                self.stdout.write(s.as_bytes())?;
                self.completation = Some(comp);
                
                cursor::holizon(&mut self.stdout, l + self.cursor as u64+ 1);
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
        cursor::holizon(&mut self.stdout, 1);
        cursor::clear_line(&mut self.stdout);
        self.stdout.write(b"\n")?;

        // panel
        for _ in 0..self.size {
            cursor::holizon(&mut self.stdout, 1);
            cursor::clear_line(&mut self.stdout);
            self.stdout.write(b"\n")?;
        }

        cursor::holizon(&mut self.stdout, 1);
        cursor::up(&mut self.stdout, (self.size + 1) as u64); // panel + input 

        Ok(())
    }

    pub fn is_last(&self) -> bool {
        self.selected == self.argument.len() - 1
    }
}
