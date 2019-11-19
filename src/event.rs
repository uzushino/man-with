pub enum Event {
    Key(char),
    ReadLine(String),
    Enter,
    Backspace,
    Delete,
    History,
    Tab,
    Forward,
    Back,
    Next,
    Prev,
    Up,
    Down,
    Left,
    Right,
    Quit,

    MoveTo(i32),

    Fn1,
    Fn2,
    Fn3,
}
