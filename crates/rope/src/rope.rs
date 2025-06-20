mod node;

use node::{Leaf, Node};
use std::fmt;
use std::rc::Rc;

#[derive(Debug)]
pub struct Rope {
    root: Rc<Node>,
}

impl Rope {
    pub fn new() -> Self {
        let leaf = Leaf::new();
        Rope {
            root: Rc::new(Node::from(leaf)),
        }
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
        self.root = self.root.insert(index, text);
        Ok(())
    }

    pub fn delete(&mut self, left_index: usize, right_index: usize) -> Result<(), DeleteError> {
        let len = self.len();
        if left_index > right_index || right_index >= len {
            return Err(DeleteError::OutOfBounds {
                left_index,
                right_index,
                len,
            });
        }
        self.root = self.root.delete(left_index, right_index);
        Ok(())
    }

    pub fn collect_leaves(&self) -> String {
        let mut buf = String::with_capacity(self.len());
        self.root.write_to(&mut buf);
        buf
    }

    // TODO: reimplement balancing for better performance
    pub fn is_balanced(&self) -> bool {
        self.root.is_balanced()
    }

    pub fn rebalanced(&self) -> Rope {
        let concatnated: &str = &self.to_string();
        Rope::from(concatnated)
    }
}

impl From<&str> for Rope {
    fn from(text: &str) -> Self {
        let mut rope = Self::new();
        rope.insert(0, text).unwrap();
        rope
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
    OutOfBounds {
        left_index: usize,
        right_index: usize,
        len: usize,
    },
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
        rope.delete(0, 5).unwrap();
        assert_eq!(rope.to_string(), "world!");
    }

    #[test]
    fn delete_at_end() {
        let mut rope = Rope::from("Hello world!");
        rope.delete(5, 11).unwrap();
        assert_eq!(rope.to_string(), "Hello");
    }

    #[test]
    fn delete_in_middle() {
        let mut rope = Rope::from("Hello beautiful world!");
        rope.delete(6, 15).unwrap();
        assert_eq!(rope.to_string(), "Hello world!");
    }

    #[test]
    fn delete_then_insert() {
        let mut rope = Rope::from("Hello beautiful world!");
        rope.delete(6, 20).unwrap();
        rope.insert(6, "world").unwrap();
        assert_eq!(rope.to_string(), "Hello world!");
    }

    #[test]
    fn insert_and_delete() {
        let mut rope = Rope::from("Hello");
        rope.insert(5, " world!").unwrap();
        rope.delete(5, 10).unwrap();
        assert_eq!(rope.to_string(), "Hello!");
    }

    #[test]
    fn correct_height() {
        let rope = Rope::from("Hello beaufitful world!");
        assert_eq!(rope.height(), 3);
    }

    #[test]
    fn correct_height_after_insert() {
        let mut rope = Rope::from("Hello world!");
        rope.insert(12, " And goodbye.").unwrap();
        assert_eq!(rope.height(), 4);
    }
}
