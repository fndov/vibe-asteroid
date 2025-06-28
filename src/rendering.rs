use std::io::{self, Write};
use log::info;
use crossterm::{
    cursor::MoveTo,
    execute,
};

// --- ScreenBuffer for simulated rendering ---
pub struct ScreenBuffer {
    pub buffer: Vec<Vec<char>>,
    pub width: u16,
    pub height: u16,
    pub cursor_x: u16,
    pub cursor_y: u16,
}

impl ScreenBuffer {
    pub fn new(width: u16, height: u16) -> Self {
        ScreenBuffer {
            buffer: vec![vec![' '; width as usize]; height as usize],
            width,
            height,
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    pub fn move_to(&mut self, x: u16, y: u16) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    pub fn write_char(&mut self, c: char) {
        if self.cursor_y < self.height && self.cursor_x < self.width {
            self.buffer[self.cursor_y as usize][self.cursor_x as usize] = c;
        }
    }

    pub fn write_str(&mut self, s: &str) {
        for c in s.chars() {
            self.write_char(c);
            self.cursor_x += 1;
        }
    }

    pub fn set_char(&mut self, x: u16, y: u16, c: char) {
        if y < self.height && x < self.width {
            self.buffer[y as usize][x as usize] = c;
        }
    }

    pub fn clear(&mut self) {
        self.buffer = vec![vec![' '; self.width as usize]; self.height as usize];
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    pub fn print_to_log(&self) {
        info!("--- Screen Buffer ---");
        for row in &self.buffer {
            info!("{}", row.iter().collect::<String>());
        }
        info!("---------------------");
    }
}

impl Write for ScreenBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        self.write_str(&s);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// --- OutputTarget enum to handle stdout or ScreenBuffer ---
pub enum OutputTarget {
    Stdout(io::Stdout),
    ScreenBuffer(ScreenBuffer),
}

impl OutputTarget {
    pub fn execute_move_to(&mut self, command: crossterm::cursor::MoveTo) -> io::Result<()> {
        match self {
            OutputTarget::Stdout(s) => execute!(s, command),
            OutputTarget::ScreenBuffer(sb) => {
                sb.move_to(command.0, command.1);
                Ok(())
            },
        }
    }

    pub fn execute_other_command(&mut self, command: impl crossterm::Command) -> io::Result<()> {
        match self {
            OutputTarget::Stdout(s) => execute!(s, command),
            OutputTarget::ScreenBuffer(_) => Ok(()), // Ignore in debug mode
        }
    }
}

impl Write for OutputTarget {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            OutputTarget::Stdout(s) => s.write(buf),
            OutputTarget::ScreenBuffer(sb) => {
                let s = String::from_utf8_lossy(buf);
                sb.write_str(&s);
                Ok(buf.len())
            },
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            OutputTarget::Stdout(s) => s.flush(),
            OutputTarget::ScreenBuffer(sb) => sb.flush(),
        }
    }
}

// --- GameGrid for geometric rendering ---
pub struct GameGrid {
    pub grid: Vec<Vec<char>>,
    pub width: u16,
    pub height: u16,
}

impl GameGrid {
    pub fn new(width: u16, height: u16) -> Self {
        GameGrid {
            grid: vec![vec![' '; width as usize]; height as usize],
            width,
            height,
        }
    }

    pub fn set_char(&mut self, x: u16, y: u16, c: char) {
        if y < self.height && x < self.width {
            self.grid[y as usize][x as usize] = c;
        }
    }

    pub fn clear(&mut self) {
        self.grid = vec![vec![' '; self.width as usize]; self.height as usize];
    }

    pub fn render(&self, stdout: &mut OutputTarget) -> io::Result<()> {
        for y in 0..self.height {
            stdout.execute_move_to(MoveTo(0, y))?;
            write!(stdout, "{}", self.grid[y as usize].iter().collect::<String>())?;
        }
        Ok(())
    }

    pub fn clear_screen_manual(&self, stdout: &mut OutputTarget, terminal_width: u16, terminal_height: u16) -> io::Result<()> {
        for y in 0..terminal_height {
            stdout.execute_move_to(MoveTo(0, y))?;
            write!(stdout, "{}", " ".repeat(terminal_width as usize))?;
        }
        stdout.execute_move_to(MoveTo(0, 0))?;
        Ok(())
    }
}

pub struct Minimap {
    buffer: Vec<Vec<char>>,
    width: u16,
    height: u16,
    x_offset: u16,
    y_offset: u16,
}

impl Minimap {
    pub fn new(width: u16, height: u16, screen_width: u16) -> Self {
        Minimap {
            buffer: vec![vec![' '; width as usize]; height as usize],
            width,
            height,
            x_offset: screen_width - width, // Top-right corner
            y_offset: 0,
        }
    }

    pub fn set_char(&mut self, x: u16, y: u16, c: char) {
        if y < self.height && x < self.width {
            self.buffer[y as usize][x as usize] = c;
        }
    }

    pub fn clear(&mut self) {
        self.buffer = vec![vec![' '; self.width as usize]; self.height as usize];
    }

    pub fn render(&self, stdout: &mut OutputTarget) -> io::Result<()> {
        for y in 0..self.height {
            stdout.execute_move_to(MoveTo(self.x_offset, self.y_offset + y))?;
            write!(stdout, "{}", self.buffer[y as usize].iter().collect::<String>())?;
        }
        Ok(())
    }
}
