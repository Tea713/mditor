use std::rc::Rc;
use std::sync::Arc;

pub const MAX_CHUNK_SIZE: usize = if cfg!(test) { 8 } else { 64 };

#[derive(Debug)]
pub enum Node {
    Branch(Branch),
    Leaf(Leaf),
}
impl Node {
    pub fn len(&self) -> usize {
        match self {
            Self::Branch(branch) => branch.len(),
            Self::Leaf(leaf) => leaf.len(),
        }
    }

    pub fn insert(&self, index: usize, text: &str) -> Node {
        match self {
            Node::Branch(branch) => branch.insert(index, text),
            Node::Leaf(leaf) => leaf.insert(index, text),
        }
    }

    pub fn collect_leaves(&self) -> String {
        match self {
            Node::Leaf(leaf) => leaf.as_str().to_owned(),
            Node::Branch(branch) => {
                let left = branch.left.collect_leaves();
                let right = branch.right.collect_leaves();
                format!("{}{}", left, right)
            }
        }
    }
}

impl From<Leaf> for Node {
    fn from(value: Leaf) -> Self {
        Node::Leaf(value)
    }
}

#[derive(Debug)]
pub struct Branch {
    left_weight: usize,
    left: Rc<Node>,
    right: Rc<Node>,
}

impl Branch {
    pub fn len(&self) -> usize {
        self.left_weight
    }

    pub fn insert(&self, index: usize, text: &str) -> Node {
        let left_weight: usize;
        let new_left: Rc<Node>;
        let new_right: Rc<Node>;
        if index < self.len() {
            left_weight = self.len() + text.len();
            new_left = Rc::new(self.left.insert(index, text));
            new_right = Rc::clone(&self.right);
        } else {
            left_weight = self.len();
            new_left = Rc::clone(&self.left);
            new_right = Rc::new(self.right.insert(index - self.len(), text));
        }
        Node::Branch(Branch {
            left_weight,
            left: new_left,
            right: new_right,
        })
    }
}

#[derive(Debug)]
pub struct Leaf {
    length: usize,
    chunk: Arc<String>,
}

impl Leaf {
    pub fn new() -> Self {
        Leaf {
            length: 0,
            chunk: Arc::new(String::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn as_str(&self) -> &str {
        &self.chunk
    }

    pub fn insert(&self, index: usize, text: &str) -> Node {
        let chunk = self.as_str();
        let (before, after) = chunk.split_at(index);
        let new_chunk = format!("{}{}{}", before, text, after);

        if new_chunk.len() > MAX_CHUNK_SIZE {
            let (left_chunk, right_chunk) = new_chunk.split_at((new_chunk.len() / 2) as usize);

            let left: Node = Leaf::new().insert(0, left_chunk);
            let right: Node = Leaf::new().insert(0, right_chunk);
            Node::Branch(Branch {
                left_weight: left_chunk.len(),
                left: Rc::new(left),
                right: Rc::new(right),
            })
        } else {
            Node::Leaf(Leaf {
                length: new_chunk.len(),
                chunk: Arc::new(new_chunk),
            })
        }
    }
}
