pub use crate::buffer::TextBuffer;

use piece_tree::StringBuffer;

#[derive(Default, Debug)]
pub struct TextBufferBuilder {
    chunks: Vec<StringBuffer>,
}

impl TextBufferBuilder {
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    /// Accept a chunk of text (may include multiple lines).
    pub fn accept_chunk(&mut self, chunk: &str) {
        if chunk.is_empty() {
            return;
        }
        self.chunks.push(StringBuffer::new(chunk.to_string()));
    }

    /// Finish building and return a `TextBuffer`.
    pub fn finish(mut self) -> TextBuffer {
        TextBuffer::from_chunks(std::mem::take(&mut self.chunks))
    }
}
