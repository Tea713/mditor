use super::error::Error;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum EditorMessage {
    NewFile,
    OpenFile,
    FileOpened(Result<(std::path::PathBuf, Vec<String>), crate::model::error::Error>),
    SaveFile,
    FileSaved(Result<(), crate::model::error::Error>),
    ActivateEditor,
    DeactivateEditor,
    SetCursor { line: usize, column: usize },
    Insert(char),
    Backspace,
    Enter,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
}
