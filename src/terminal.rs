pub struct Size {
    pub columns: u16,
    pub rows: u16,
}

pub struct Terminal {
    size: Size,
}

impl Terminal {
    pub fn default() -> Result<Self, std::io::Error> {
        let size = crossterm::terminal::size()?;
        Ok(Self{
            size: Size{
                columns: size.0,
                rows: size.1.saturating_sub(2),
            },
        })
    }

    pub fn size(&self) -> &Size {
        &self.size
    }
}