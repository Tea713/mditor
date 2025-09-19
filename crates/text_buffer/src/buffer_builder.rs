use crate::buffer::TextBuffer;
use piece_tree::StringBuffer;
use std::{
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

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

    pub fn load_from_path<P: AsRef<Path>>(path: P) -> io::Result<TextBuffer> {
        let chunks = Self::read_chunks_from_path(path)?;
        let mut builder = TextBufferBuilder::new();
        for s in chunks {
            builder.accept_chunk(&s);
        }
        Ok(builder.finish())
    }

    pub fn read_chunks_from_path<P: AsRef<Path>>(path: P) -> io::Result<Vec<String>> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let mut out: Vec<String> = Vec::new();
        let mut buf = vec![0u8; 64 * 1024];
        let mut carry: Vec<u8> = Vec::new();

        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }

            // Combine carry + new bytes
            let mut combined = Vec::with_capacity(carry.len() + n);
            combined.extend_from_slice(&carry);
            combined.extend_from_slice(&buf[..n]);

            // Find longest valid UTF-8 prefix
            let valid_len = match std::str::from_utf8(&combined) {
                Ok(_) => combined.len(),
                Err(e) => e.valid_up_to(),
            };

            if valid_len > 0 {
                let s = std::str::from_utf8(&combined[..valid_len]).expect("valid UTF-8 prefix");
                out.push(s.to_string());
            }

            // Keep any partial codepoint for the next read
            carry.clear();
            if valid_len < combined.len() {
                carry.extend_from_slice(&combined[valid_len..]);
            }
        }

        if !carry.is_empty() {
            match std::str::from_utf8(&carry) {
                Ok(s) => out.push(s.to_string()),
                Err(_) => {
                    // lossy decode trailing broken sequence
                    out.push(String::from_utf8_lossy(&carry).to_string());
                }
            }
        }

        Ok(out)
    }
}
