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
        let empty_leaf = Rc::new(Node::Leaf(Leaf::from("")));
        Rope { root: empty_leaf }
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
        match Branch::create_root(&branches).first() {
            Some(node) => self.root = Rc::clone(&node),
            None => self.root = Rc::new(Node::Leaf(Leaf::from(""))),
        }
        Ok(())
    }

    pub fn delete(&mut self, range: Range<usize>) -> Result<(), DeleteError> {
        let len = self.len();
        if range.end > len {
            return Err(DeleteError::OutOfBounds { range, len });
        }
        let branches = self.root.delete(range);
        match Branch::create_root(&branches).first() {
            Some(node) => self.root = Rc::clone(&node),
            None => self.root = Rc::new(Node::Leaf(Leaf::from(""))),
        }
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
        if text.is_empty() {
            return Rope::new();
        }
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

    #[test]
    fn empty_rope_operations() {
        let mut rope = Rope::new();
        assert_eq!(rope.len(), 0);
        assert_eq!(rope.to_string(), "");

        rope.insert(0, "Hello").unwrap();
        assert_eq!(rope.to_string(), "Hello");
        assert_eq!(rope.len(), 5);

        rope.delete(0..5).unwrap();
        assert_eq!(rope.to_string(), "");
        assert_eq!(rope.len(), 0);

        rope.insert(0, "World").unwrap();
        assert_eq!(rope.to_string(), "World");
    }

    #[test]
    fn empty_string_from() {
        let rope = Rope::from("");
        assert_eq!(rope.len(), 0);
        assert_eq!(rope.to_string(), "");
        assert_eq!(rope.height(), 1);
    }

    #[test]
    fn single_character_operations() {
        let mut rope = Rope::from("a");
        assert_eq!(rope.len(), 1);

        rope.insert(0, "b").unwrap();
        assert_eq!(rope.to_string(), "ba");

        rope.insert(2, "c").unwrap();
        assert_eq!(rope.to_string(), "bac");

        rope.delete(1..2).unwrap();
        assert_eq!(rope.to_string(), "bc");
    }

    #[test]
    fn insert_out_of_bounds() {
        let mut rope = Rope::from("Hello");

        let result = rope.insert(6, " World");
        assert!(result.is_err());

        assert_eq!(rope.to_string(), "Hello");
    }

    #[test]
    fn delete_out_of_bounds() {
        let mut rope = Rope::from("Hello");

        let result = rope.delete(0..6);
        assert!(result.is_err());

        let result = rope.delete(6..7);
        assert!(result.is_err());

        assert_eq!(rope.to_string(), "Hello");
    }

    #[test]
    fn empty_range_delete() {
        let mut rope = Rope::from("Hello");

        // Delete empty range
        rope.delete(2..2).unwrap();
        assert_eq!(rope.to_string(), "Hello");

        // Delete at end
        rope.delete(5..5).unwrap();
        assert_eq!(rope.to_string(), "Hello");
    }

    #[test]
    fn insert_empty_string() {
        let mut rope = Rope::from("Hello");
        rope.insert(2, "").unwrap();
        assert_eq!(rope.to_string(), "Hello");
    }

    #[test]
    fn large_string_creation() {
        let large_text = "a".repeat(1000);
        let rope = Rope::from(large_text.as_str());
        assert_eq!(rope.to_string(), large_text);
        assert_eq!(rope.len(), 1000);

        assert!(rope.height() > 1);
    }

    #[test]
    fn large_insert() {
        let mut rope = Rope::from("Hello");
        let large_insert = "x".repeat(500);

        rope.insert(2, &large_insert).unwrap();

        let expected = format!("He{}llo", large_insert);
        assert_eq!(rope.to_string(), expected);
        assert_eq!(rope.len(), 505);
    }

    #[test]
    fn large_delete() {
        let large_text = "a".repeat(1000);
        let mut rope = Rope::from(large_text.as_str());

        // Delete most of it
        rope.delete(100..900).unwrap();

        let expected = "a".repeat(100) + &"a".repeat(100);
        assert_eq!(rope.to_string(), expected);
        assert_eq!(rope.len(), 200);
    }

    // TODO: fix issues where splitting leaves split inside emojis
    #[test]
    fn unicode_characters() {
        let text = "Hello 🌍 World! 你好";
        let mut rope = Rope::from(text);

        // Length should be in bytes, not chars
        assert_eq!(rope.len(), text.len());
        assert_eq!(rope.to_string(), text);

        // Insert unicode
        rope.insert(6, "🦀 ").unwrap();
        assert_eq!(rope.to_string(), "Hello 🦀 🌍 World! 你好");
    }

    #[test]
    fn emoji_operations() {
        let mut rope = Rope::from("👨‍👩‍👧‍👦");
        let original_len = rope.len();

        rope.insert(0, "Family: ").unwrap();
        assert_eq!(rope.len(), original_len + "Family: ".len());

        // Make sure the emoji is still intact
        assert!(rope.to_string().contains("👨‍👩‍👧‍👦"));
    }

    #[test]
    fn many_small_inserts() {
        let mut rope = Rope::new();

        for i in 0..100 {
            rope.insert(i, "x").unwrap();
        }

        assert_eq!(rope.len(), 100);
        assert_eq!(rope.to_string(), "x".repeat(100));
    }

    #[test]
    fn many_small_deletes() {
        let text = "x".repeat(100);
        let mut rope = Rope::from(text.as_str());

        // Delete from the end, one character at a time
        for i in (0..100).rev() {
            rope.delete(i..i + 1).unwrap();
        }

        assert_eq!(rope.len(), 0);
        assert_eq!(rope.to_string(), "");
    }

    #[test]
    fn alternating_insert_delete() {
        let mut rope = Rope::from("base");

        for _ in 0..50 {
            // Insert
            rope.insert(2, "xx").unwrap();
            assert!(rope.len() >= 4);

            // Delete part of what we just inserted
            if rope.len() > 4 {
                rope.delete(2..3).unwrap();
            }
        }
        assert!(rope.len() >= 4);
    }

    #[test]
    fn length_consistency() {
        let mut rope = Rope::from("Hello World");
        let initial_len = rope.len();

        // After insert
        rope.insert(5, " Beautiful").unwrap();
        assert_eq!(rope.len(), initial_len + " Beautiful".len());

        // After delete
        rope.delete(5..15).unwrap(); // Remove " Beautiful"
        assert_eq!(rope.len(), initial_len);
        assert_eq!(rope.to_string(), "Hello World");
    }

    #[test]
    fn height_reasonableness() {
        // Small rope should have small height
        let small_rope = Rope::from("Hello");
        assert!(small_rope.height() <= 2);

        // Large rope should have reasonable height (logarithmic)
        let large_text = "a".repeat(10000);
        let large_rope = Rope::from(large_text.as_str());
        assert!(large_rope.height() > 1);
        assert!(large_rope.height() < 20); // Should not be too deep
    }

    #[test]
    fn insert_preserves_existing_content() {
        let original = "Hello World";
        let mut rope = Rope::from(original);

        rope.insert(6, "Beautiful ").unwrap();

        let result = rope.to_string();
        assert!(result.starts_with("Hello "));
        assert!(result.ends_with("World"));
        assert!(result.contains("Beautiful"));
    }

    #[test]
    fn delete_only_removes_specified_range() {
        let original = "0123456789";
        let mut rope = Rope::from(original);

        rope.delete(3..7).unwrap(); // Remove "3456"

        let result = rope.to_string();
        assert_eq!(result, "012789");
        assert!(!result.contains("3456"));
    }

    #[test]
    fn rope_equals_string_after_operations() {
        let text = "Hello Beautiful World";
        let mut rope = Rope::from(text);
        let mut string = String::from(text);

        // Perform same operations on both
        rope.delete(6..16).unwrap();
        string.replace_range(6..16, "");
        assert_eq!(rope.to_string(), string);

        rope.insert(6, "Wonderful ").unwrap();
        string.insert_str(6, "Wonderful ");
        assert_eq!(rope.to_string(), string);
    }

    #[test]
    fn delete_across_chunk_boundaries() {
        // Create a rope that will definitely span multiple chunks
        let text = "a".repeat(100); // This will create multiple chunks
        let mut rope = Rope::from(text.as_str());

        // Delete across chunk boundaries
        rope.delete(10..90).unwrap();

        let expected = "a".repeat(10) + &"a".repeat(10);
        assert_eq!(rope.to_string(), expected);
        assert_eq!(rope.len(), 20);
    }

    #[test]
    fn insert_at_chunk_boundaries() {
        let text = "a".repeat(100);
        let mut rope = Rope::from(text.as_str());

        rope.insert(8, "X").unwrap();
        rope.insert(16, "Y").unwrap();
        rope.insert(50, "Z").unwrap();

        let result = rope.to_string();
        assert!(result.contains("X"));
        assert!(result.contains("Y"));
        assert!(result.contains("Z"));
        assert_eq!(rope.len(), 103);
    }

    #[test]
    fn tree_structure_after_complex_operations() {
        let mut rope = Rope::from("a".repeat(1000).as_str());

        rope.delete(100..900).unwrap();
        rope.insert(50, &"b".repeat(500)).unwrap();
        rope.delete(200..400).unwrap();

        assert!(rope.height() > 0);
        assert!(rope.height() < 15); // Not too deep

        let result = rope.to_string();
        assert_eq!(result.len(), rope.len());
    }

    #[test]
    fn a_bunch_of_operations() {
        let text = "djfh;ldjhfak93[ 21i pejk;lkwen c;msdnf;ow
            en 3krj;l2k3  v 234 234312333523
            4]34 vjkdjl;k  pw3rpioj2[p3oij4bnbxlwer]sdj; lk23,";
        let mut rope = Rope::from(text);
        let mut string = String::from(text);

        let mut to_insert = "lkdajs;ldij34   2ij3;l12nnn
                    mdfn.ln erewr werereeee  erernnnnn nermwnernnnmewrn
                    asdkjlkw3jpuidpqw
                    ksckwke daskjdlkajsre dsfkr";

        rope.insert(6, to_insert).unwrap();
        string.insert_str(6, to_insert);
        assert_eq!(rope.to_string(), string);

        rope.delete(6..16).unwrap();
        string.replace_range(6..16, "");
        assert_eq!(rope.to_string(), string);

        to_insert = " asdasdccc     w3qrdw
            asjdhlkhff
            g  gfgfgg rteroi";

        rope.delete(25..39).unwrap();
        string.replace_range(25..39, "");
        assert_eq!(rope.to_string(), string);

        rope.insert(45, to_insert).unwrap();
        string.insert_str(45, to_insert);
        assert_eq!(rope.to_string(), string);
    }
}
