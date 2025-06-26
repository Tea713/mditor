use std::ops::Range;
use std::{cmp, rc::Rc};

pub const MAX_CHUNK_SIZE: usize = if cfg!(test) { 8 } else { 64 };
pub const TREE_ORDER: usize = 4;

#[derive(Debug)]
pub enum Node {
    Branch(Branch),
    Leaf(Leaf),
}

impl Node {
    pub fn is_leaf(&self) -> bool {
        match self {
            Self::Branch(_) => false,
            Self::Leaf(_) => true,
        }
    }
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

    pub fn children(&self) -> Vec<Rc<Node>> {
        match self {
            Self::Branch(branch) => branch.children.clone(),
            Self::Leaf(_) => Vec::new(),
        }
    }

    pub fn insert(&self, index: usize, text: &str) -> Vec<Rc<Node>> {
        match self {
            Self::Branch(branch) => {
                let branches = branch.insert(index, text);
                Branch::create_root(&branches)
            }
            Self::Leaf(leaf) => leaf.insert(index, text),
        }
    }

    pub fn delete(&self, range: Range<usize>) -> Vec<Rc<Node>> {
        match self {
            Self::Branch(branch) => {
                let node_list = branch.delete(range);
                let mut root = Rc::clone(node_list.first().unwrap());
                if root.len() == 0 {
                    let leaves = Leaf::split_text_to_leaves("");
                    let empty = Rc::clone(Branch::create_root(&leaves).first().unwrap());
                    return vec![empty];
                }
                let mut children = root.children();
                let mut first_child = Rc::clone(children.first().unwrap());
                while !first_child.is_leaf() && children.len() == 1 {
                    root = Rc::clone(&first_child);
                    children = root.children();
                    first_child = Rc::clone(children.first().unwrap());
                }
                vec![Rc::clone(&root)]
            }
            Self::Leaf(leaf) => leaf.delete(range),
        }
    }

    pub fn write_to(&self, buf: &mut String) {
        match self {
            Self::Branch(branch) => {
                for child in branch.children.clone() {
                    child.write_to(buf);
                }
            }
            Self::Leaf(leaf) => buf.push_str(leaf.as_str()),
        }
    }
}

#[derive(Debug)]
pub struct Branch {
    height: usize,
    length: usize,
    keys: Vec<usize>,
    children: Vec<Rc<Node>>,
}

impl Branch {
    pub fn height(&self) -> usize {
        self.height
    }

    pub fn len(&self) -> usize {
        self.length
    }

    // create parent branch(es) for a bunch of branches
    pub fn create_parent_branches(children: &Vec<Rc<Node>>) -> Vec<Rc<Node>> {
        let mut parents: Vec<Rc<Node>> = Vec::new();

        if children.is_empty() {
            return parents;
        }

        let num_parents = (children.len() as f64 / TREE_ORDER as f64).ceil() as usize;
        let parent_capacity = (children.len() as f64 / num_parents as f64).ceil() as usize;

        for chunk in children.chunks(parent_capacity) {
            let branch_children = chunk.to_vec();
            let mut keys: Vec<usize> = Vec::new();
            let mut length: usize = 0;

            for child in chunk.iter().take(chunk.len().saturating_sub(1)) {
                length += child.len();
                keys.push(length);
            }

            if let Some(last_child) = chunk.last() {
                length += last_child.len();
            }
            parents.push(Rc::new(Node::Branch(Branch {
                children: branch_children,
                height: children.first().unwrap().height() + 1,
                keys,
                length,
            })))
        }
        parents
    }

    //create parent branches recursively to get a root
    pub fn create_root(nodes: &Vec<Rc<Node>>) -> Vec<Rc<Node>> {
        if nodes.is_empty() {
            return Vec::new();
        }
        if nodes.len() == 1 {
            return nodes.clone();
        }
        let parents = Branch::create_parent_branches(&nodes.clone());
        return Branch::create_root(&parents);
    }

    // return the index of the child and the real index in the child
    pub fn find_child_by_index(&self, index: usize) -> (usize, usize) {
        let mut offset = 0;
        for (pos, key) in self.keys.iter().enumerate() {
            if index < *key {
                return (pos, index - offset);
            };
            offset = *key;
        }
        return (self.children.len() - 1, index - offset);
    }

    // return the indexes of the children and the real ranges in the them
    pub fn find_children_by_range(&self, range: Range<usize>) -> Vec<(usize, Range<usize>)> {
        let mut result: Vec<(usize, Range<usize>)> = Vec::new();
        let mut start: usize = 0;
        let mut end: usize;
        for (pos, key) in self.keys.iter().enumerate() {
            end = *key;
            if range.start < end && start < range.end {
                let real_range: Range<usize> =
                    (cmp::max(range.start, start) - start)..(cmp::min(range.end, end) - start);
                result.push((pos, real_range));
            }
            start = end;
        }
        end = self.len();
        if range.start < end && start < range.end {
            let real_range: Range<usize> =
                (cmp::max(range.start, start) - start)..(cmp::min(range.end, end) - start);
            result.push((self.children.len() - 1, real_range));
        }
        result
    }

    pub fn insert(&self, index: usize, text: &str) -> Vec<Rc<Node>> {
        let (insert_index, index_in_child) = self.find_child_by_index(index);
        let mut children = self.children.clone();
        let inserted_node = Rc::clone(&children[insert_index]);

        let new_children = inserted_node.insert(index_in_child, text);
        children.splice(insert_index..=insert_index, new_children);

        Self::create_parent_branches(&children)
    }

    pub fn delete(&self, range: Range<usize>) -> Vec<Rc<Node>> {
        let to_delete = self.find_children_by_range(range);

        if to_delete.is_empty() {
            return Self::create_parent_branches(&self.children);
        }

        let mut children = self.children.clone();
        let mut altered_children: Vec<Rc<Node>> = Vec::new();

        for (pos, range_in_child) in &to_delete {
            let altered_node = Rc::clone(&children[*pos]);
            let mut altered = altered_node.delete(range_in_child.clone());
            altered_children.append(&mut altered);
        }
        let start = to_delete.first().unwrap().0;
        let end = to_delete.last().unwrap().0;
        children.splice(start..=end, altered_children);
        if children.first().unwrap().is_leaf() {
            return Self::create_parent_branches(&children);
        }
        let mut children_of_children: Vec<Rc<Node>> = Vec::new();
        for child in &children {
            children_of_children.append(&mut child.children());
        }
        children = Self::create_parent_branches(&children_of_children);
        Self::create_parent_branches(&children)
    }
}

impl From<Leaf> for Branch {
    fn from(value: Leaf) -> Self {
        Branch {
            height: 2,
            length: value.len(),
            keys: Vec::new(),
            children: vec![Rc::new(Node::Leaf(value))],
        }
    }
}

#[derive(Debug)]
pub struct Leaf {
    chunk: String,
}

impl From<&str> for Leaf {
    fn from(value: &str) -> Self {
        Leaf {
            chunk: value.to_owned(),
        }
    }
}

impl Leaf {
    pub fn as_str(&self) -> &str {
        &self.chunk
    }

    pub fn len(&self) -> usize {
        self.chunk.len()
    }

    pub fn split_text_to_leaves(text: &str) -> Vec<Rc<Node>> {
        if text.is_empty() {
            return Vec::new();
        }
        let mut leaves: Vec<Rc<Node>> = Vec::new();

        let num_split = (text.len() as f64 / MAX_CHUNK_SIZE as f64).ceil() as usize;
        let chunk_size = (text.len() as f64 / num_split as f64).ceil() as usize;

        for i in 0..num_split {
            let start = chunk_size * i;
            let end = cmp::min(chunk_size * (i + 1), text.len());
            let chunk = &text[start..end];
            let new_leaf = Rc::new(Node::Leaf(Leaf::from(chunk)));
            leaves.push(new_leaf);
        }
        leaves
    }

    pub fn insert(&self, index: usize, text: &str) -> Vec<Rc<Node>> {
        let (before, after) = self.chunk.split_at(index);
        let new_text: String = format!("{}{}{}", before, text, after);
        Self::split_text_to_leaves(&new_text)
    }

    pub fn delete(&self, range: Range<usize>) -> Vec<Rc<Node>> {
        let mut new_text = self.chunk.to_owned();
        new_text.replace_range(range, "");
        Self::split_text_to_leaves(&new_text)
    }
}
