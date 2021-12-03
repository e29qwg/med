use crate::Row;
use crate::Position;

use std::io::{Write, BufRead, BufReader, BufWriter};

#[derive(Default)]
pub struct Document {
    rows: Vec<Row>,
    path: Option<String>,
    dirty: bool,
}

impl Document {
    pub fn open(path: &str) -> Result<Self, std::io::Error> {
        let mut rows = Vec::new();
        let f = std::fs::File::open(path).expect("Unable to open file");
        let f = BufReader::new(f);
        
        for value in f.lines() {
            rows.push(Row::from(&value.expect("Unable to read file")));
        }

        Ok(Self {
            rows,
            path: Some(path.to_string()),
            dirty: false,
        })
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(path) = &self.path {
            let file = std::fs::File::create(path)?;
            let mut file = BufWriter::new(file);
            for row in &self.rows {
                file.write_all(row.as_bytes())?;
                file.write_all(b"\n")?;
            }
        }
        Ok(())
    } 

    pub fn row(&self, index: usize) -> Option<&Row> {
        self.rows.get(index)
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn insert_newline(&mut self, at: &Position) {
        if at.row > self.rows.len() {
            return;
        }

        self.dirty = true;

        if at.row == self.rows.len() {
            self.rows.push(Row::default());
            return;
        }
        
        let new_row = self.rows[at.row].split(at.col);
        self.rows.insert(at.row + 1, new_row);
    }

    pub fn insert(&mut self, at: &Position, c: char) {
        if at.row > self.rows.len() {
            return;
        }

        self.dirty = true;

        if c == '\n' {
            self.insert_newline(at);
            return;
        }
        if at.row == self.rows.len() {
            let mut row = Row::default();
            row.insert(0, c);
            self.rows.push(row);
        } else {
            let row = &mut self.rows[at.row];
            row.insert(at.col, c);
        }
    }

    pub fn delete(&mut self, at: &Position) {
        let len = self.rows.len();

        if at.row >= len {
            return;
        }

        self.dirty = true;

        if at.col == self.rows[at.row].len() && at.row + 1 < len {
            let next_row = self.rows.remove(at.row + 1);
            let row = &mut self.rows[at.row];
            row.append(&next_row);
        } else {
            let row = &mut self.rows[at.row];
            row.delete(at.col);
        }
    }
}