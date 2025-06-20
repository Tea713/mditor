use std::cmp::max;
use std::rc::Rc;

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

    pub fn height(&self) -> usize {
        match self {
            Self::Branch(branch) => branch.height(),
            Self::Leaf(_) => 1,
        }
    }

    pub fn is_balanced(&self) -> bool {
        match self {
            Self::Branch(branch) => {
                if branch.left.height().abs_diff(branch.right.height()) > 5 {
                    false
                } else {
                    branch.left.is_balanced() && branch.right.is_balanced()
                }
            }
            Self::Leaf(_) => true,
        }
    }

    pub fn insert(&self, index: usize, text: &str) -> Rc<Node> {
        match self {
            Node::Branch(branch) => branch.insert(index, text),
            Node::Leaf(leaf) => leaf.insert(index, text),
        }
    }

    pub fn delete(&self, left_index: usize, right_index: usize) -> Rc<Node> {
        match self {
            Node::Branch(branch) => branch.delete(left_index, right_index),
            Node::Leaf(leaf) => leaf.delete(left_index, right_index),
        }
    }

    pub fn write_to(&self, buf: &mut String) {
        match self {
            Node::Leaf(leaf) => buf.push_str(leaf.as_str()),
            Node::Branch(branch) => {
                branch.left.write_to(buf);
                branch.right.write_to(buf);
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
    right_weight: usize,
    height: usize,
    left: Rc<Node>,
    right: Rc<Node>,
}

impl Branch {
    pub fn len(&self) -> usize {
        self.left_weight + self.right_weight
    }

    pub fn left_weight(&self) -> usize {
        self.left_weight
    }

    pub fn right_weight(&self) -> usize {
        self.right_weight
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn insert(&self, index: usize, text: &str) -> Rc<Node> {
        let left_weight: usize;
        let right_weight: usize;
        let new_left: Rc<Node>;
        let new_right: Rc<Node>;
        if index <= self.left_weight() {
            left_weight = self.left_weight() + text.len();
            right_weight = self.right_weight();
            new_left = self.left.insert(index, text);
            new_right = Rc::clone(&self.right);
        } else {
            left_weight = self.left_weight();
            right_weight = self.right_weight() + text.len();
            new_left = Rc::clone(&self.left);
            new_right = self.right.insert(index - self.left_weight(), text);
        }
        Rc::new(Node::Branch(Branch {
            left_weight,
            right_weight,
            height: 1 + max(new_left.height(), new_right.height()),
            left: new_left,
            right: new_right,
        }))
    }

    pub fn delete(&self, left_index: usize, right_index: usize) -> Rc<Node> {
        if left_index == 0 && right_index == self.len() - 1 {
            return Rc::new(Node::Leaf(Leaf::new()));
        }

        let deleted_len = right_index - left_index + 1;

        let left_weight: usize;
        let right_weight: usize;
        let new_left: Rc<Node>;
        let new_right: Rc<Node>;
        if right_index < self.left_weight() {
            left_weight = self.left_weight() - deleted_len;
            right_weight = self.right_weight();
            new_left = self.left.delete(left_index, right_index);
            new_right = Rc::clone(&self.right);
        } else if left_index >= self.left_weight() {
            left_weight = self.left_weight();
            right_weight = self.right_weight() - deleted_len;
            new_left = Rc::clone(&self.left);
            new_right = self.right.delete(
                left_index - self.left_weight(),
                right_index - self.left_weight(),
            );
        } else {
            left_weight = left_index;
            right_weight = self.len() - right_index - 1;
            new_left = self.left.delete(left_index, self.left_weight() - 1);
            new_right = self.right.delete(0, right_index - self.left_weight());
        }
        Rc::new(Node::Branch(Branch {
            left_weight,
            right_weight,
            height: 1 + max(new_left.height(), new_right.height()),
            left: new_left,
            right: new_right,
        }))
    }
}

#[derive(Debug)]
pub struct Leaf {
    length: usize,
    chunk: Rc<String>,
}

impl Leaf {
    pub fn new() -> Self {
        Leaf {
            length: 0,
            chunk: Rc::new(String::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn as_str(&self) -> &str {
        &self.chunk
    }

    pub fn insert(&self, index: usize, text: &str) -> Rc<Node> {
        let chunk = self.as_str();
        let (before, after) = chunk.split_at(index);
        let new_chunk = format!("{}{}{}", before, text, after);

        if new_chunk.len() > MAX_CHUNK_SIZE {
            let (left_chunk, right_chunk) = new_chunk.split_at((new_chunk.len() / 2) as usize);

            let left: Rc<Node> = Leaf::new().insert(0, left_chunk);
            let right: Rc<Node> = Leaf::new().insert(0, right_chunk);
            Rc::new(Node::Branch(Branch {
                left_weight: left_chunk.len(),
                right_weight: right_chunk.len(),
                height: 1 + max(left.height(), right.height()),
                left,
                right,
            }))
        } else {
            Rc::new(Node::Leaf(Leaf {
                length: new_chunk.len(),
                chunk: Rc::new(new_chunk),
            }))
        }
    }

    pub fn delete(&self, left_index: usize, right_index: usize) -> Rc<Node> {
        let chunk = self.as_str();
        let before = &chunk[..left_index];
        let after = &chunk[(right_index + 1)..];
        let new_chunk = format!("{}{}", before, after);
        Rc::new(Node::Leaf(Leaf {
            length: new_chunk.len(),
            chunk: Rc::new(new_chunk),
        }))
    }
}
