use std::io::Write;

pub fn up<W>(out: &mut W, n: u64)
where
    W: Write,
{
    let _ = write!(out, "\x1b[{}A", n);
}

pub fn down<W>(out: &mut W, n: u64)
where
    W: Write,
{
    let _ = write!(out, "\x1b[{}B", n);
}

#[allow(dead_code)]
pub fn forward<W>(out: &mut W, n: u64)
where
    W: Write,
{
    let _ = write!(out, "\x1b[{}C", n);
}

#[allow(dead_code)]
pub fn back<W>(out: &mut W, n: u64)
where
    W: Write,
{
    let _ = write!(out, "\x1b[{}D", n);
}

#[allow(dead_code)]
pub fn next_line<W>(out: &mut W, n: u64)
where
    W: Write,
{
    let _ = write!(out, "\x1b[{}E", n);
}

#[allow(dead_code)]
pub fn previous_line<W>(out: &mut W, n: u64)
where
    W: Write,
{
    let _ = write!(out, "\x1b[{}F", n);
}

pub fn horizon<W>(out: &mut W, n: u64)
where
    W: Write,
{
    let _ = write!(out, "\x1b[{}G", n);
}

#[allow(dead_code)]
pub fn show<W>(out: &mut W)
where
    W: Write,
{
    let _ = write!(out, "\x1b[?25h");
}

#[allow(dead_code)]
pub fn hide<W>(out: &mut W)
where
    W: Write,
{
    let _ = write!(out, "\x1b[?25l");
}

pub fn clear_line<W>(out: &mut W)
where
    W: Write,
{
    let _ = write!(out, "\x1b[K");
}
