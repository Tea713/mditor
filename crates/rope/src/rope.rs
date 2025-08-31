mod node;

use node::Node;
use std::ops::Range;
use std::rc::Rc;
use std::{cmp, fmt};

#[derive(Debug)]
pub struct Rope {
    node: Rc<Node>,
}

impl Rope {
    pub fn new() -> Self {
        Rope { node: Node::new() }
    }

    pub fn len(&self) -> usize {
        self.node.len()
    }

    pub fn is_empty(&self) -> bool {
        self.node.len() == 0
    }

    pub fn height(&self) -> usize {
        self.node.height()
    }

    pub fn new_lines(&self) -> usize {
        self.node.new_lines()
    }

    pub fn insert(&mut self, index: usize, text: &str) {
        self.node = self.node.insert(cmp::min(index, self.len()), text);
    }

    pub fn delete(&mut self, range: Range<usize>) {
        self.node = self
            .node
            .delete(cmp::min(range.start, self.len())..cmp::min(range.end, self.len()));
    }

    pub fn slice(&self, range: Range<usize>) -> Self {
        Rope {
            node: self
                .node
                .slice(range.start..cmp::min(range.end, self.len())),
        }
    }

    pub fn chunks(&self) -> ChunkIter {
        ChunkIter::new(self)
    }

    pub fn lines(&self) -> impl Iterator<Item = String> {
        self.node.lines()
    }

    // TODO: lines, columnes conversion to integrate to editor

    pub fn collect_leaves(&self) -> String {
        let mut buf = String::with_capacity(self.len());
        self.node.write_to(&mut buf);
        buf
    }
}

impl From<&str> for Rope {
    fn from(text: &str) -> Self {
        if text.is_empty() {
            return Rope::new();
        }
        Rope {
            node: Node::from_str(text),
        }
    }
}

impl fmt::Display for Rope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.collect_leaves())
    }
}

impl Default for Rope {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ChunkIter {
    stack: Vec<Rc<Node>>,
}

impl ChunkIter {
    fn new(rope: &Rope) -> Self {
        let mut iter = Self { stack: Vec::new() };
        iter.stack.push(Rc::clone(&rope.node));
        iter
    }
}

impl Iterator for ChunkIter {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.stack.pop() {
            match node.as_ref() {
                Node::Leaf(leaf) => return Some(leaf.as_str().to_string()),
                Node::Branch(branch) => {
                    for child in branch.children().iter().rev() {
                        self.stack.push(Rc::clone(child));
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: should probably manually reduce the number of test while making tests more high quality, maybe introduce some randomness?

    #[test]
    fn lines_iter() {
        let hello_vec: Vec<String> = vec![
            "Hello world!".to_string(),
            "rweklrj; fefwert".to_string(),
            "rkkkkew ffwerrtwqwr dddae3414cc".to_string(),
        ];

        let hello_rope =
            Rope::from("Hello world!\nrweklrj; fefwert\nrkkkkew ffwerrtwqwr dddae3414cc");

        let iter_vec: Vec<String> = hello_rope.lines().collect();

        assert_eq!(hello_vec, iter_vec);
    }

    #[test]
    fn empty_lines_iter() {
        let new_lines_vec: Vec<String> = vec![
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ];

        let new_lines_rope = Rope::from("\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n");

        let iter_vec: Vec<String> = new_lines_rope.lines().collect();

        assert_eq!(new_lines_vec, iter_vec);
    }

    #[test]
    fn new_lines_count() {
        let mut hello_string = String::from(
            "
            Hello world!
            I am a rope.
            Yaddi asdjjdasf fdsadjjjjj
            dasg ewwertdasdff
            293481 12oi3jlkjjjdjla lllasd
            ",
        );
        let mut hello_rope = Rope::from(hello_string.as_str());
        assert_eq!(hello_rope.new_lines(), hello_string.matches('\n').count());

        let rope_slice = hello_rope.slice(40..67);
        let str_slice = &hello_string[40..67];

        assert_eq!(rope_slice.new_lines(), str_slice.matches('\n').count());

        hello_rope.delete(10..29);
        hello_string.replace_range(10..29, "");

        assert_eq!(hello_rope.new_lines(), hello_string.matches('\n').count());

        hello_rope.insert(
            37,
            "
            dsakj
            dsdfl;wwww dad
            ddddd ddddd  daasdw
            ",
        );

        hello_string.insert_str(
            37,
            "
            dsakj
            dsdfl;wwww dad
            ddddd ddddd  daasdw
            ",
        );

        assert_eq!(hello_rope.new_lines(), hello_string.matches('\n').count());
    }

    #[test]
    fn slicing() {
        let hello_rope = Rope::from("Hello world! I am a rope.");
        let hello_slice = hello_rope.slice(0..12);
        assert_eq!(hello_slice.to_string(), "Hello world!");
    }

    #[test]
    fn same() {
        let hello_rope = Rope::from("Hello world! I am a rope.");
        assert_eq!(hello_rope.to_string(), "Hello world! I am a rope.");
    }

    #[test]
    fn different() {
        let hello_rope = Rope::from("Hello world! I am a rope.");
        assert_ne!(hello_rope.to_string(), "Hello world! I am a string.");
    }

    #[test]
    fn insert_at_beginning() {
        let mut rope = Rope::from("world! I am a rope.");
        rope.insert(0, "Hello ");
        assert_eq!(rope.to_string(), "Hello world! I am a rope.");
    }

    #[test]
    fn insert_at_end() {
        let mut rope = Rope::from("Hello");
        rope.insert(5, " world! I am a rope.");
        assert_eq!(rope.to_string(), "Hello world! I am a rope.");
    }

    #[test]
    fn insert_in_middle() {
        let mut rope = Rope::from("Helloworld!Iamarope.");
        rope.insert(5, " ");
        rope.insert(12, " ");
        rope.insert(14, " ");
        rope.insert(17, " ");
        rope.insert(19, " ");
        assert_eq!(rope.to_string(), "Hello world! I am a rope.");
    }

    #[test]
    fn delete_at_beginning() {
        let mut rope = Rope::from("Hello world!");
        rope.delete(0..6);
        assert_eq!(rope.to_string(), "world!");
    }

    #[test]
    fn delete_at_end() {
        let mut rope = Rope::from("Hello world!");
        rope.delete(5..12);
        assert_eq!(rope.to_string(), "Hello");
    }

    #[test]
    fn delete_in_middle() {
        let mut rope = Rope::from("Hello beautiful world!");
        rope.delete(6..16);
        assert_eq!(rope.to_string(), "Hello world!");
    }

    #[test]
    fn delete_then_insert() {
        let mut rope = Rope::from("Hello beautiful world!");
        rope.delete(6..21);
        rope.insert(6, "world");
        assert_eq!(rope.to_string(), "Hello world!");
    }

    #[test]
    fn insert_and_delete() {
        let mut rope = Rope::from("Hello");
        rope.insert(5, " world!");
        rope.delete(5..11);
        assert_eq!(rope.to_string(), "Hello!");
    }

    #[test]
    fn empty_rope_operations() {
        let mut rope = Rope::new();
        assert_eq!(rope.len(), 0);
        assert_eq!(rope.to_string(), "");

        rope.insert(0, "Hello");
        assert_eq!(rope.to_string(), "Hello");
        assert_eq!(rope.len(), 5);

        rope.delete(0..5);
        assert_eq!(rope.to_string(), "");
        assert_eq!(rope.len(), 0);

        rope.insert(0, "World");
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

        rope.insert(0, "b");
        assert_eq!(rope.to_string(), "ba");

        rope.insert(2, "c");
        assert_eq!(rope.to_string(), "bac");

        rope.delete(1..2);
        assert_eq!(rope.to_string(), "bc");
    }

    #[test]
    fn insert_out_of_bounds() {
        let mut rope = Rope::from("Hello");
        rope.insert(6, " World");
        assert_eq!(rope.to_string(), "Hello World");
    }

    #[test]
    fn delete_out_of_bounds() {
        let mut rope = Rope::from("Hello");
        rope.delete(0..6);
        rope.delete(6..7);
        assert_eq!(rope.to_string(), "");
    }

    #[test]
    fn empty_range_delete() {
        let mut rope = Rope::from("Hello");

        rope.delete(2..2);
        assert_eq!(rope.to_string(), "Hello");

        rope.delete(5..5);
        assert_eq!(rope.to_string(), "Hello");
    }

    #[test]
    fn insert_empty_string() {
        let mut rope = Rope::from("Hello");
        rope.insert(2, "");
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

        rope.insert(2, &large_insert);

        let expected = format!("He{large_insert}llo");
        assert_eq!(rope.to_string(), expected);
        assert_eq!(rope.len(), 505);
    }

    #[test]
    fn large_delete() {
        let large_text = "a".repeat(1000);
        let mut rope = Rope::from(large_text.as_str());

        rope.delete(100..900);

        let expected = "a".repeat(100) + &"a".repeat(100);
        assert_eq!(rope.to_string(), expected);
        assert_eq!(rope.len(), 200);
    }

    #[test]
    fn unicode_characters() {
        let text = "Hello 🌍 World! 你好";
        let mut rope = Rope::from(text);

        rope.insert(6, "🦀 ");
        assert_eq!(rope.to_string(), "Hello 🦀 🌍 World! 你好");
    }

    #[test]
    fn emoji_operations() {
        let mut rope = Rope::from("👨‍👩‍👧‍👦");
        let original_len = rope.len();

        rope.insert(0, "Family: ");
        assert_eq!(rope.len(), original_len + "Family: ".len());

        assert!(rope.to_string().contains("👨‍👩‍👧‍👦"));
    }

    #[test]
    fn many_small_inserts() {
        let mut rope = Rope::new();

        for i in 0..100 {
            rope.insert(i, "x");
        }

        assert_eq!(rope.len(), 100);
        assert_eq!(rope.to_string(), "x".repeat(100));
    }

    #[test]
    fn many_small_deletes() {
        let text = "x".repeat(100);
        let mut rope = Rope::from(text.as_str());

        for i in (0..100).rev() {
            rope.delete(i..i + 1);
        }

        assert_eq!(rope.len(), 0);
        assert_eq!(rope.to_string(), "");
    }

    #[test]
    fn alternating_insert_delete() {
        let mut rope = Rope::from("base");

        for _ in 0..50 {
            rope.insert(2, "xx");
            assert!(rope.len() >= 4);

            if rope.len() > 4 {
                rope.delete(2..3);
            }
        }
        assert!(rope.len() >= 4);
    }

    #[test]
    fn length_consistency() {
        let mut rope = Rope::from("Hello World");
        let initial_len = rope.len();

        // After insert
        rope.insert(5, " Beautiful");
        assert_eq!(rope.len(), initial_len + " Beautiful".len());

        // After delete
        rope.delete(5..15); // Remove " Beautiful"
        assert_eq!(rope.len(), initial_len);
        assert_eq!(rope.to_string(), "Hello World");
    }

    #[test]
    fn height_reasonableness() {
        let small_rope = Rope::from("Hello");
        assert!(small_rope.height() <= 2);

        let large_text = "a".repeat(10000);
        let large_rope = Rope::from(large_text.as_str());
        assert!(large_rope.height() > 1);
        assert!(large_rope.height() < 20);
    }

    #[test]
    fn insert_preserves_existing_content() {
        let original = "Hello World";
        let mut rope = Rope::from(original);

        rope.insert(6, "Beautiful ");

        let result = rope.to_string();
        assert!(result.starts_with("Hello "));
        assert!(result.ends_with("World"));
        assert!(result.contains("Beautiful"));
    }

    #[test]
    fn delete_only_removes_specified_range() {
        let original = "0123456789";
        let mut rope = Rope::from(original);

        rope.delete(3..7); // Remove "3456"

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
        rope.delete(6..16);
        string.replace_range(6..16, "");
        assert_eq!(rope.to_string(), string);

        rope.insert(6, "Wonderful ");
        string.insert_str(6, "Wonderful ");
        assert_eq!(rope.to_string(), string);
    }

    #[test]
    fn delete_across_chunk_boundaries() {
        let text = "a".repeat(100);
        let mut rope = Rope::from(text.as_str());

        rope.delete(10..90);

        let expected = "a".repeat(10) + &"a".repeat(10);
        assert_eq!(rope.to_string(), expected);
        assert_eq!(rope.len(), 20);
    }

    #[test]
    fn insert_at_chunk_boundaries() {
        let text = "a".repeat(100);
        let mut rope = Rope::from(text.as_str());

        rope.insert(8, "X");
        rope.insert(16, "Y");
        rope.insert(50, "Z");

        let result = rope.to_string();
        assert!(result.contains("X"));
        assert!(result.contains("Y"));
        assert!(result.contains("Z"));
        assert_eq!(rope.len(), 103);
    }

    #[test]
    fn tree_structure_after_complex_operations() {
        let mut rope = Rope::from("a".repeat(1000).as_str());

        rope.delete(100..900);
        rope.insert(50, &"b".repeat(500));
        rope.delete(200..400);

        assert!(rope.height() > 0);
        assert!(rope.height() < 15);

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

        rope.insert(6, to_insert);
        string.insert_str(6, to_insert);
        assert_eq!(rope.to_string(), string);

        rope.delete(6..16);
        string.replace_range(6..16, "");
        assert_eq!(rope.to_string(), string);

        let res = rope.node.check_leaves_same_depths();
        match res {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{err}");
                panic!("Not same depth");
            }
        }

        to_insert = " asdasdccc     w3qrdw
            asjdhlkhff
            g  gfgfgg rteroi";

        rope.delete(25..39);
        string.replace_range(25..39, "");
        assert_eq!(rope.to_string(), string);

        rope.insert(45, to_insert);
        string.insert_str(45, to_insert);
        assert_eq!(rope.to_string(), string);

        let res = rope.node.check_leaves_same_depths();
        match res {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{err}");
                panic!("Not same depth!")
            }
        }
    }
}
