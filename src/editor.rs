const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

use std::{io::{stdout, Write}, time::Duration};

use futures::{future::FutureExt, select, StreamExt};
use futures_timer::Delay;

use crate::Terminal;
use crate::Document;
use crate::Row;

use crossterm::{
    cursor,
    event::{Event, EventStream, KeyCode, KeyModifiers, KeyEvent},
    execute, queue,
    style,
    terminal::{disable_raw_mode, enable_raw_mode,
        EnterAlternateScreen, LeaveAlternateScreen,
        Clear, ClearType},
    Result,
};

#[derive(Default)]
pub struct Position {
    pub col: usize,
    pub row: usize,
}

struct StatusMessage {
    text: String,
}
impl StatusMessage {
    fn from(message: String) -> Self {
        Self {
            text: message,
        }
    }
}

pub struct Editor {
    stdout: std::io::Stdout,
    events: crossterm::event::EventStream,
    terminal: Terminal,
    document: Document,
    cursor_position: Position,
    offset: Position,
    status_message: StatusMessage,
    is_running: bool,
}

impl Editor {
    pub fn default() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let document = if args.len() > 1 {
            let path = &args[1];
            Document::open(path).unwrap_or_default()
        } else {
            Document::default()
        };

        Self {
            stdout: stdout(),
            terminal: Terminal::default().expect("failed initializing terminal"),
            document,
            events: EventStream::new(),
            cursor_position: Position::default(),
            offset: Position::default(),
            status_message: StatusMessage::from("".to_string()),
            is_running: true,
        }
    }

    fn render_welcome_message(&self) -> String {
        let mut welcome_message = format!("{} v{}", NAME, VERSION);
        let cols = self.terminal.size().columns as usize;
        let len = welcome_message.len();
        let padding = cols.saturating_sub(len) / 2;
        let spaces = " ".repeat(padding.saturating_sub(1));

        welcome_message = format!("~{}{}", spaces, welcome_message);
        welcome_message.truncate(cols);

        welcome_message + "\r\n"
    }

    fn render_status_bar(&self) -> String{
        let mut status: String = self.status_message.text.clone();
        let width = self.terminal.size().columns as usize;

        let line_indicator = format!(
            "{},{}",
            self.cursor_position.row.saturating_add(1),
            self.document.len()
        );
        let len = status.len() + line_indicator.len();

        if width > len {
            status.push_str(&" ".repeat(width - len));
        }

        status = format!("{}{}", status, line_indicator);
        status.truncate(width);
        
        status
    }

    fn render_row(&self, row: &Row) -> String {
        let width = self.terminal.size().columns as usize;
        let start = self.offset.col;
        let end = self.offset.col + width;
        let row = row.render(start, end);

        row + "\r\n"
    }

    fn draw_rows(&mut self) -> Result<()> {
        let rows = self.terminal.size().rows;
        let mut row_string: String;

        for terminal_row in 0..rows {
            queue!(
                self.stdout,
                Clear(ClearType::CurrentLine)
            )?;

            if let Some(row) = self.document.row(terminal_row as usize + self.offset.row) {
                row_string = self.render_row(row);
            } else if self.document.is_empty() && terminal_row == rows / 3 {
                row_string = self.render_welcome_message();
            } else {
                row_string = String::from("~\r\n");
            }

            queue!(
                self.stdout,
                style::Print(row_string)
            )?;
        }

        let status_bar = self.render_status_bar();
        queue!(
            self.stdout,
            style::Print(status_bar)
        )
    }

    fn set_cursor_position(&mut self, pos: &Position) -> Result<()> {
        queue!(
            self.stdout,
            cursor::MoveTo(pos.col as u16, pos.row as u16)
        )
    }

    fn refresh_screen(&mut self) -> Result<()> {
        queue!(
            self.stdout,
            cursor::Hide,
            cursor::MoveTo(0, 0),
        )?;

        if self.is_running {
            self.draw_rows()?;
            self.set_cursor_position(&Position {
                col: self.cursor_position.col.saturating_sub(self.offset.col),
                row: self.cursor_position.row.saturating_sub(self.offset.row),
            })?;
        } else {
            queue!(self.stdout, Clear(ClearType::All))?;
        }

        queue!(
            self.stdout,
            cursor::Show,
        )?;

        self.stdout.flush()
    }

    fn scroll(&mut self) {
        let Position { col, row } = self.cursor_position;
        let width = self.terminal.size().columns as usize;
        let height = self.terminal.size().rows as usize;
        let mut offset = &mut self.offset;

        if row < offset.row {
            offset.row = row;
        } else if row >= offset.row.saturating_add(height) {
            offset.row = row.saturating_sub(height).saturating_add(1);
        }

        if col < offset.col {
            offset.col = col;
        } else if col >= offset.col.saturating_add(width) {
            offset.col = col.saturating_sub(width).saturating_add(1);
        }
    }

    fn move_cursor(&mut self, key: KeyCode) {
        let terminal_rows = self.terminal.size().rows as usize;
        let Position { mut col, mut row } = self.cursor_position;
        let height = self.document.len();

        let mut width = if let Some(row) = self.document.row(row) {
            row.len()
        } else {
            0
        };

        match key {
            KeyCode::Up => row = row.saturating_sub(1),
            KeyCode::Down => {
                if row < height {
                    row = row.saturating_add(1);
                }
            }
            KeyCode::Left => {
                if col > 0 {
                    col -= 1;
                } else if row > 0 {
                    row -= 1;
                    if let Some(row) = self.document.row(row) {
                        col = row.len();
                    } else {
                        col = 0;
                    }
                }
            }
            KeyCode::Right => {
                if col < width {
                    col += 1;
                } else if row < height {
                    row += 1;
                    col = 0;
                }
            }
            KeyCode::PageUp => {
                row = if row > terminal_rows {
                    row - terminal_rows
                } else {
                    0
                }
            },
            KeyCode::PageDown => {
                row = if row.saturating_add(terminal_rows) < height {
                    row + terminal_rows as usize
                } else {
                    height
                }
            },
            KeyCode::Home => col = 0,
            KeyCode::End => col = width,
            _ => (),
        }

        width = if let Some(row) = self.document.row(row) {
            row.len()
        } else {
            0
        };
        
        col = std::cmp::min(col, width);

        self.cursor_position = Position { col, row }
    }

    async fn process_key(&mut self, event: KeyEvent) -> Result<()> {
        match event {
            KeyEvent {code: KeyCode::Up, modifiers: KeyModifiers::NONE}
            | KeyEvent {code: KeyCode::Down, modifiers: KeyModifiers::NONE}
            | KeyEvent {code: KeyCode::Left, modifiers: KeyModifiers::NONE}
            | KeyEvent {code: KeyCode::Right, modifiers: KeyModifiers::NONE}
            | KeyEvent {code: KeyCode::Home, modifiers: KeyModifiers::NONE}
            | KeyEvent {code: KeyCode::End, modifiers: KeyModifiers::NONE}
            | KeyEvent {code: KeyCode::PageUp, modifiers: KeyModifiers::NONE}
            | KeyEvent {code: KeyCode::PageDown, modifiers: KeyModifiers::NONE} => {
                self.move_cursor(event.code);
            },
            KeyEvent {code: KeyCode::Enter, ..} => {
                self.document.insert_newline(&self.cursor_position);
                self.move_cursor(KeyCode::Down);
                self.move_cursor(KeyCode::Home);
            },
            KeyEvent {code: KeyCode::Delete, ..} => self.document.delete(&self.cursor_position),
            KeyEvent {code: KeyCode::Backspace, ..} => {
                if self.cursor_position.col > 0 || self.cursor_position.row > 0 {
                    self.move_cursor(KeyCode::Left);
                    self.document.delete(&self.cursor_position);
                }
            },
            KeyEvent {code: KeyCode::Esc, modifiers: KeyModifiers::NONE} => {
                self.is_running = false;
            },
            KeyEvent {code: KeyCode::Char('s'), modifiers: KeyModifiers::CONTROL } => {
                if self.document.save().is_ok() {
                    self.status_message =
                    StatusMessage::from("File written.".to_string());
                } else {
                    self.status_message = StatusMessage::from("Unable to write the file!".to_string());
                }
            },
            KeyEvent {code: KeyCode::Char(c), ..} => {
                self.document.insert(&self.cursor_position, c);
                self.move_cursor(KeyCode::Right);
            },
            _ => ()
        }

        self.scroll();
        Ok(())
    }

    async fn process_events(&mut self) -> Result<()> {
        let mut delay = Delay::new(Duration::from_millis(1_000)).fuse();
        let mut event = self.events.next().fuse();
        
        select! {
            _ = delay => Ok(()),
            maybe_event = event => {
                match maybe_event {
                    Some(Ok(event)) => {
                        match event {
                            Event::Key(key) => self.process_key(key).await,
                            Event::Mouse(_event) => Ok(()),
                            Event::Resize(_width, _height) => Ok(()),
                        }
                    }
                    Some(Err(e)) => Err(e),
                    None => {
                        self.is_running = false;
                        Ok(())
                    },
                }
            },
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        execute!(stdout(), EnterAlternateScreen)?;
        enable_raw_mode()?;

        queue!(
            self.stdout,
            cursor::MoveTo(0, 0),
        )?;

        while self.is_running {
            if let Err(error) = self.refresh_screen() {
                die(error);
            }

            self.process_events().await?
        }

        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)
    }
}

fn die(e: crossterm::ErrorKind) {
    let _error = execute!(
        stdout(),
        Clear(ClearType::All),
    );

    panic!("{}", e);
}
