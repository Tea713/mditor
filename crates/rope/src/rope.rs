mod node;

use node::{Leaf, Node};

#[derive(Debug)]
pub struct Rope {
    root: Node,
}

impl Rope {
    pub fn new() -> Self {
        let leaf = Leaf::new();
        Rope {
            root: Node::from(leaf),
        }
    }

    pub fn len(&self) -> usize {
        self.root.len()
    }

    pub fn insert(&mut self, index: usize, text: &str) {
        self.root = self.root.insert(index, text);
    }

    pub fn to_string(&self) -> String {
        self.root.collect_leaves()
    }

    // TODO: self balancing

    // pub fn is_balanced(&self) -> bool {}

    // pub fn rebalanced(&mut self) {}
}

impl From<&str> for Rope {
    fn from(text: &str) -> Self {
        let mut rope = Self::new();
        rope.insert(0, text);
        rope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_world() {
        let hello_rope =
            Rope::from("Hello world! Blah blah blah. Blah a lot. Random things at the end.");
        let hello_string =
            String::from("Hello world! Blah blah blah. Blah a lot. Random things at the end.");
        assert_eq!(hello_rope.to_string(), hello_string);
    }

    #[test]
    fn hello_not_the_same() {
        let hello_rope = Rope::from("Hello world! Blah blah blah. Random things at the end.");
        let hello_string = String::from(
            "Hello not the same! Blah bluh blah. Random things in the end but different.",
        );
        assert_ne!(hello_rope.to_string(), hello_string);
    }
}
