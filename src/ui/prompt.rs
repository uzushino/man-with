use std::io::Write;
use std::process::Command;

use termion;
use terminal_size::terminal_size;

use crate::ui::cursor;

const PROMPT: &'static str = "PROMPT > ";

#[derive(Clone)]
pub struct Prompt<T: Write + Send + Drop> {
    pub panel: Vec<String>,
    pub input: String,
    pub command: String,
    pub argument: Vec<String>,
    pub stdout: T,
    buffer: Vec<String>,
    cursor: usize,
    pos: usize,
    size: usize,
}

impl<T: Write + Send + Drop> Prompt<T> {
    pub fn new(stdout: T, command: &str, height: usize) -> Self {
        let cmd = Command::new("sh")
            .arg("-c")
            .arg(format!("man {} | col -bx", command))
            .output()
            .expect("failed to execute process");

        let out = cmd.stdout;
        let s = String::from_utf8_lossy(&out);

        Prompt {
            panel: vec![String::new();height],
            input: String::new(),
            command: String::from(command),
            argument: Vec::new(),
            stdout: stdout,
            buffer: s.split('\n').map(|v| v.to_string()).collect(),
            cursor: 0,
            pos: 0,
            size: height,
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

    fn viewpoint(&self) -> (usize, usize) {
        let (s, e) = if self.pos + self.size > self.buffer.len() {
                      ((self.buffer.len() as isize) - (self.size as isize), self.buffer.len() as isize)
                    } else {
                        (self.pos as isize, (self.pos + self.size) as isize)
                    };
        let s = if s < 0 {
            0
        } else {
            s
        };
        
        (s as usize, e as usize)
    }

    pub fn backspace(&mut self) {
        if let Some(ch) = self.input[0..self.cursor].chars().rev().next() {
            self.cursor -= ch.len_utf8();
            self.input.remove(self.cursor);

            if let Some(n) = self.find_position(&self.buffer) {
                self.pos = n;
            }
        }
    }

    pub fn append(&mut self) {
        self.argument.push(self.input.clone());
        self.input = String::new();
        self.cursor = 0;
    }

    pub fn insert(&mut self, ch: char) {
        self.input.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();

        if let Some(n) = self.find_position(&self.buffer) {
            self.pos = n;
        }
    }

    pub fn find_position(&self, buffer: &Vec<String>) -> Option<usize> {
        let n = &self.input;
        buffer.iter().position(|v| v.contains(n))
    }

    pub fn prompt() -> String {
        format!("{}{}{}", termion::style::Bold, PROMPT, termion::style::Reset)
    }

    pub fn show_command(&mut self) -> Result<(), std::io::Error> {
        write!(self.stdout, "COMMAND> {} {}", self.command, self.argument.join(" "))
    }
    
    pub fn show_input(&mut self) -> Result<usize, std::io::Error> {
        let p = format!("{}{}", Self::prompt(), self.input);
        self.stdout.write(p.as_bytes())
    }

    pub fn show_candidates(&mut self) -> Vec<String> {
        let (s, e) = self.viewpoint();
        let mut buffer = self.buffer.clone();
        let decorated = format!("{red}{input}{reset}", 
            red = termion::color::Fg(termion::color::Red), 
            input = self.input, 
            reset = termion::style::Reset);
        buffer[self.pos] = buffer[self.pos].replace(&self.input, &decorated);

        let lines = &buffer[s..e];
        for l in lines {
            self.stdout.write(l.as_bytes()).unwrap();
            
            cursor::down(&mut self.stdout, 1);
            cursor::holizon(&mut self.stdout, 1);
        };

        Vec::from(lines)
    }

    pub fn show(&mut self) -> Result<(), failure::Error> {
        self.sweep()?;

        let size = terminal_size();

        if let Some(_) = size {
            self.show_input()?;
            cursor::down(&mut self.stdout, 1);
            cursor::holizon(&mut self.stdout, 1 as u64);

            self.show_command()?;
            cursor::down(&mut self.stdout, 1);
            cursor::holizon(&mut self.stdout, 1 as u64);

            let lines = self.show_candidates();
            cursor::up(&mut self.stdout, lines.len() as u64);

            // Move cursor input position.
            cursor::up(&mut self.stdout, 2u64);
            cursor::holizon(&mut self.stdout, (PROMPT.len() + self.input.len() + 1) as u64);
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), failure::Error> {
        self.stdout.flush()?;
        Ok(())
    }

    pub fn sweep(&mut self) -> Result<(), failure::Error> {
        // command
        cursor::holizon(&mut self.stdout, 1);
        cursor::clear_line(&mut self.stdout);
        self.stdout.write(b"\n")?;

        // input
        cursor::holizon(&mut self.stdout, 1);
        cursor::clear_line(&mut self.stdout);
        self.stdout.write(b"\n")?;

        // panel
        for _ in 0..self.size {
            cursor::holizon(&mut self.stdout, 1);
            cursor::clear_line(&mut self.stdout);
            self.stdout.write(b"\n")?;
        };

        cursor::holizon(&mut self.stdout, 1);
        cursor::up(&mut self.stdout, (self.size + 2) as u64); // panel + input + command

        Ok(())
    }
}
