use super::error::Error;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum EditorMessage {
    NewFile,
    OpenFile,
    FileOpened(Result<(PathBuf, Vec<String>), Error>),
    SaveFile,
    FileSaved(Result<(), Error>),
    ActivateEditor,
    DeactivateEditor,
    SetCursor { line: usize, column: usize },
    Insert(String),
    Backspace,
    Enter,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
}
