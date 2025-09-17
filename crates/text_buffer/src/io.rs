use std::{
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

use crate::TextBufferBuilder;
use crate::buffer::TextBuffer;

pub fn load_from_path<P: AsRef<Path>>(path: P) -> io::Result<TextBuffer> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut builder = TextBufferBuilder::new();
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

        // Push valid part as chunk
        if valid_len > 0 {
            let s = std::str::from_utf8(&combined[..valid_len]).expect("valid UTF-8 prefix");
            builder.accept_chunk(s);
        }

        // Keep remainder (possibly a partial codepoint) for next read
        carry.clear();
        if valid_len < combined.len() {
            carry.extend_from_slice(&combined[valid_len..]);
        }
    }

    // Flush any remaining carry
    if !carry.is_empty() {
        match std::str::from_utf8(&carry) {
            Ok(s) => builder.accept_chunk(s),
            Err(_) => {
                // Fallback: lossy decode trailing broken sequence
                let s = String::from_utf8_lossy(&carry);
                builder.accept_chunk(&s);
            }
        }
    }

    Ok(builder.finish())
}
