use std::process::Command;

#[derive(Clone)]
pub enum SourceType {
    Man,
    Help,
}

#[derive(Clone, PartialEq)]
pub enum ShowType {
    Normal,
    LineNumber,
}

#[derive(Clone)]
pub struct Viewer {
  source_type: SourceType,
  show_type: ShowType,
  command: String,
}

impl Viewer {
  pub fn new(command: &str, source_type: SourceType) -> Self {
    Viewer {
      source_type: source_type,
      show_type: ShowType::Normal,
      command: String::from(command)
    }
  }

  pub fn source(&self) -> String {
    match self.source_type {
      SourceType::Man => self.man(),
      SourceType::Help => self.help(),
    }
  }

  pub fn show(&self, buffer: Vec<String>) -> Vec<String> {
      match self.show_type {
          ShowType::LineNumber => {
            buffer
              .iter()
              .enumerate()
              .map(|(i, line)| format!("{number} {line}", number = i + 1, line = line))
              .collect::<Vec<_>>()
          }
          _ => buffer,
      }
  }

  pub fn toggle_show_type(&mut self, typ: ShowType) {
    self.show_type = if self.show_type == typ {
        ShowType::Normal
    } else {
        typ
    }
  }

  fn man(&self) -> String {
    let cmd = Command::new("sh")
        .arg("-c")
        .arg(format!("man {} | col -bx", self.command))
        .output()
        .expect("failed to execute process");
    let out = cmd.stdout;

    String::from_utf8_lossy(&out).to_string()
  }
  
  fn help(&self) -> String {
    let cmd = Command::new(self.command.clone())
      .arg("--help")
      .output()
      .expect("failed to execute process");
    let out = cmd.stdout;

    String::from_utf8_lossy(&out).to_string()
  }
}