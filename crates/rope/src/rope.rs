mod node;

use node::{Branch, Leaf, Node};
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

#[derive(Debug)]
pub struct Rope {
    root: Rc<Node>,
}

impl Rope {
    pub fn new() -> Self {
        let leaves = Leaf::split_text_to_leaves("");
        let root = Rc::clone(Branch::create_root(&leaves).first().unwrap());
        Rope { root }
    }

    pub fn len(&self) -> usize {
        self.root.len()
    }

    pub fn height(&self) -> usize {
        self.root.height()
    }

    pub fn insert(&mut self, index: usize, text: &str) -> Result<(), InsertError> {
        let len = self.len();
        if index > len {
            return Err(InsertError::OutOfBounds { index, len });
        }
        let branches = self.root.insert(index, text);
        self.root = Rc::clone(&Branch::create_root(&branches).first().unwrap());
        Ok(())
    }

    pub fn delete(&mut self, range: Range<usize>) -> Result<(), DeleteError> {
        let len = self.len();
        if range.end > len {
            return Err(DeleteError::OutOfBounds { range, len });
        }
        let branches = self.root.delete(range);
        self.root = Rc::clone(&Branch::create_root(&branches).first().unwrap());
        Ok(())
    }

    pub fn collect_leaves(&self) -> String {
        let mut buf = String::with_capacity(self.len());
        self.root.write_to(&mut buf);
        buf
    }
}

impl From<&str> for Rope {
    fn from(text: &str) -> Self {
        let leaves = Leaf::split_text_to_leaves(text);
        let root = Rc::clone(Branch::create_root(&leaves).first().unwrap());
        Rope { root }
    }
}

impl fmt::Display for Rope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.collect_leaves())
    }
}

#[derive(Debug)]
pub enum InsertError {
    OutOfBounds { index: usize, len: usize },
}

#[derive(Debug)]
pub enum DeleteError {
    OutOfBounds { range: Range<usize>, len: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_world() {
        let hello_rope = Rope::from("Hello world!");
        let hello_string = String::from("Hello world!");
        assert_eq!(hello_rope.to_string(), hello_string);
    }

    #[test]
    fn hello_not_the_same() {
        let hello_rope = Rope::from("Hello rope!");
        let hello_string = String::from("Hello word!");
        assert_ne!(hello_rope.to_string(), hello_string);
    }

    #[test]
    fn insert_at_beginning() {
        let mut rope = Rope::from("world!");
        rope.insert(0, "Hello ").unwrap();
        assert_eq!(rope.to_string(), "Hello world!");
    }

    #[test]
    fn insert_at_end() {
        let mut rope = Rope::from("Hello");
        rope.insert(5, " world!").unwrap();
        assert_eq!(rope.to_string(), "Hello world!");
    }

    #[test]
    fn insert_in_middle() {
        let mut rope = Rope::from("Helloworld!");
        rope.insert(5, " ").unwrap();
        assert_eq!(rope.to_string(), "Hello world!");
    }

    #[test]
    fn delete_at_beginning() {
        let mut rope = Rope::from("Hello world!");
        rope.delete(0..6).unwrap();
        assert_eq!(rope.to_string(), "world!");
    }

    #[test]
    fn delete_at_end() {
        let mut rope = Rope::from("Hello world!");
        rope.delete(5..12).unwrap();
        assert_eq!(rope.to_string(), "Hello");
    }

    #[test]
    fn delete_in_middle() {
        let mut rope = Rope::from("Hello beautiful world!");
        rope.delete(6..16).unwrap();
        assert_eq!(rope.to_string(), "Hello world!");
    }

    #[test]
    fn delete_then_insert() {
        let mut rope = Rope::from("Hello beautiful world!");
        rope.delete(6..21).unwrap();
        rope.insert(6, "world").unwrap();
        assert_eq!(rope.to_string(), "Hello world!");
    }

    #[test]
    fn insert_and_delete() {
        let mut rope = Rope::from("Hello");
        rope.insert(5, " world!").unwrap();
        rope.delete(5..11).unwrap();
        assert_eq!(rope.to_string(), "Hello!");
    }
}
