use std::str::FromStr;

use piece_tree::{BufferCursor, PieceTree, StringBuffer};

/// Public alias for positions (1-based line/column), forwarded from piece_tree.
pub type Position = BufferCursor;

#[derive(Debug)]
pub struct TextBuffer {
    tree: PieceTree,
}

impl TextBuffer {
    /// Build from multiple chunks
    pub fn from_chunks(mut chunks: Vec<StringBuffer>) -> Self {
        let tree = PieceTree::new(chunks.as_mut_slice());
        Self { tree }
    }

    /// Insert `value` at byte `offset` in the document.
    pub fn insert(&mut self, offset: usize, value: &str) {
        self.tree.insert(offset, value);
    }

    /// Delete `len` bytes starting at byte `offset`.
    pub fn delete(&mut self, offset: usize, len: usize) {
        self.tree.delete(offset, len);
    }

    /// Convenience: insert at (line, column), both 1-based.
    pub fn insert_at(&mut self, line: usize, column: usize, value: &str) {
        let off = self.get_offset_at(line, column);
        self.insert(off, value);
    }

    /// Convenience: delete a range specified by start (line, column) and length in bytes.
    pub fn delete_at(&mut self, line: usize, column: usize, len: usize) {
        let off = self.get_offset_at(line, column);
        self.delete(off, len);
    }

    /// Get complete text content.
    pub fn get_text(&self) -> String {
        self.tree.get_text()
    }

    /// Get the number of lines (1-based; empty doc => 1 line).
    pub fn get_line_count(&self) -> usize {
        self.tree.line_count()
    }

    /// Get the document byte length.
    pub fn get_length(&self) -> usize {
        self.tree.len()
    }

    /// Get content of a line (1-based). Out-of-range => empty.
    pub fn get_line_content(&self, line_number: usize) -> String {
        self.tree.get_line_content(line_number)
    }

    /// Get all lines (without EOL).
    pub fn get_lines_content(&self) -> Vec<String> {
        self.tree.get_lines_content()
    }

    /// Get the byte length (without EOL) of a line (1-based).
    pub fn get_line_length(&self, line_number: usize) -> usize {
        self.tree.get_line_length(line_number)
    }

    /// 1-based (line, column) to 0-based byte offset.
    pub fn get_offset_at(&self, line_number: usize, column: usize) -> usize {
        self.tree.get_offset_at(line_number, column)
    }

    /// 0-based byte offset to 1-based position.
    pub fn get_position_at(&self, offset: usize) -> Position {
        self.tree.get_position_at(offset)
    }

    /// UI-friendly: max column on a line (1-based).
    pub fn get_line_max_column(&self, line_number: usize) -> usize {
        self.get_line_length(line_number) + 1
    }
}

#[derive(Debug)]
pub struct ParseError;

impl FromStr for TextBuffer {
    type Err = ParseError;

    /// Build from a single string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let chunk = StringBuffer::new(s.to_string());
        Ok(Self::from_chunks(vec![chunk]))
    }
}
