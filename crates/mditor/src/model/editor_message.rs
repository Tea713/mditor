use super::error::Error;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum EditorMessage {
    NewFile,
    OpenFile,
    FileOpened(Result<(PathBuf, Vec<String>), Error>),
    SaveFile,
    SaveAs,
    FileSaved(Result<Option<PathBuf>, Error>),
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
    BeginSelection { line: usize, column: usize },
    ExtendSelectionTo { line: usize, column: usize },
    EndSelection,
    SelectAll,
    DeleteForward,
    ExtendLeft,
    ExtendRight,
    ExtendUp,
    ExtendDown,
}
