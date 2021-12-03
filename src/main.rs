mod document;
mod editor;
mod row;
mod terminal;
pub use document::Document;
use editor::Editor;
pub use editor::Position;
pub use row::Row;
pub use terminal::Terminal;

use crossterm::Result;

#[tokio::main]
async fn main() -> Result<()> {
    Editor::default().run().await
}
