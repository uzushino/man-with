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

    Beginning,

    Fn1,
    Fn2,
    Fn3,
}
