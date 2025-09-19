use super::error::Error;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum EditorMessage {
    // TODO: define actions of the editor
    ActionPerformed,
    NewFile,
    OpenFile,
    FileOpened(Result<(PathBuf, Vec<String>), Error>),
    SaveFile,
    FileSaved(Result<PathBuf, Error>),
}
