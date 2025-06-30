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
    pub fn new() -> Rc<Self> {
        Rc::new(Node::Leaf(Leaf::new()))
    }

    pub fn from_str(value: &str) -> Rc<Self> {
        let leaves = Leaf::split_text_to_leaves(value);
        Rc::clone(&Self::create_root(&leaves))
    }

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

    pub fn insert(&self, index: usize, text: &str) -> Rc<Node> {
        let nodes = self.insert_recursive(index, text);
        return Rc::clone(&Self::create_root(&nodes));
    }

    pub fn insert_recursive(&self, index: usize, text: &str) -> Vec<Rc<Node>> {
        match self {
            Self::Branch(branch) => branch.insert(index, text),
            Self::Leaf(leaf) => leaf.insert(index, text),
        }
    }

    pub fn delete(&self, range: Range<usize>) -> Rc<Node> {
        let nodes = self.delete_recursive(range);
        let root = Node::truncate_root(&nodes);
        Rc::clone(&root)
    }

    pub fn delete_recursive(&self, range: Range<usize>) -> Vec<Rc<Node>> {
        match self {
            Self::Branch(branch) => branch.delete(range),
            Self::Leaf(leaf) => leaf.delete(range),
        }
    }

    // create parent branch(es) for node(s)
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

    // create parent branches until a root that support all provided branches is formed
    pub fn create_root(nodes: &Vec<Rc<Node>>) -> Rc<Node> {
        let mut curr_nodes = nodes.clone();
        while curr_nodes.len() > 1 {
            curr_nodes = Node::create_parent_branches(&curr_nodes);
        }
        match curr_nodes.first() {
            None => Self::new(),
            Some(root) => Rc::clone(&root),
        }
    }

    // remove nodes that are not necessary for the tree to have all of its data by traversing to the left
    // currently just used after deletion when it leaves a series of nodes from root to a certain nodes that each have a single child
    pub fn truncate_root(nodes: &Vec<Rc<Node>>) -> Rc<Node> {
        let mut curr_nodes = nodes.clone();
        while !curr_nodes.is_empty() {
            let root = Rc::clone(curr_nodes.first().unwrap());
            if root.is_leaf() {
                return root;
            }
            let children = root.children();
            if children.len() > 1 {
                return root;
            }
            curr_nodes = children;
        }
        return Self::new();
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

    // Just a help function to make sure a leaves are at the same height
    #[allow(dead_code)]
    pub fn check_leaves_same_depths(&self) -> Result<(), String> {
        let mut leaf_depths = Vec::new();
        self.collect_leaf_depths(&mut leaf_depths, 1);

        if leaf_depths.is_empty() {
            return Ok(());
        }

        let first_height = leaf_depths[0];
        if leaf_depths.iter().all(|&height| height == first_height) {
            Ok(())
        } else {
            let min_height = *leaf_depths.iter().min().unwrap();
            let max_height = *leaf_depths.iter().max().unwrap();
            Err(format!(
                "Leaves at inconsistent heights: min={}, max={}, found heights: {:?}",
                min_height, max_height, leaf_depths
            ))
        }
    }

    // Helper of that help function above
    #[allow(dead_code)]
    fn collect_leaf_depths(&self, depths: &mut Vec<usize>, curr_depth: usize) {
        match self {
            Self::Leaf(_) => depths.push(curr_depth),
            Self::Branch(branch) => {
                for child in &branch.children {
                    child.collect_leaf_depths(depths, curr_depth + 1);
                }
            }
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

    // recursively find the correct child to insert into and create new nodes while keeping unaffected nodes
    pub fn insert(&self, index: usize, text: &str) -> Vec<Rc<Node>> {
        let (insert_index, index_in_child) = self.find_child_by_index(index);
        let mut children = self.children.clone();
        let inserted_node = Rc::clone(&children[insert_index]);

        let new_children = inserted_node.insert_recursive(index_in_child, text);
        children.splice(insert_index..=insert_index, new_children);

        Node::create_parent_branches(&children)
    }

    // recursively find the correct children to delete and keep unaffected nodes
    pub fn delete(&self, range: Range<usize>) -> Vec<Rc<Node>> {
        let mut children = self.children.clone();

        // The previous recursion call likely deleted everything in this branch
        if children.is_empty() {
            return Vec::new();
        }

        let to_delete = self.find_children_by_range(range);

        if to_delete.is_empty() {
            return Node::create_parent_branches(&self.children);
        }

        let mut altered_children: Vec<Rc<Node>> = Vec::new();

        for (pos, range_in_child) in &to_delete {
            let altered_node = Rc::clone(&children[*pos]);
            let altered = altered_node.delete_recursive(range_in_child.clone());
            altered_children.extend(altered);
        }

        let start = to_delete.first().unwrap().0;
        let end = to_delete.last().unwrap().0;
        children.splice(start..=end, altered_children);

        if children.is_empty() {
            return Vec::new();
        }

        // No need to check if the children of the current is filled less than half its max capacity when children are leaves
        if children.first().unwrap().is_leaf() {
            return Node::create_parent_branches(&children);
        }

        let mut need_restructure = false;
        let mut grandchildren: Vec<Rc<Node>> = Vec::new();
        for child in &children {
            grandchildren.extend(child.children());
            if child.children().len() < TREE_ORDER / 2 {
                need_restructure = true;
            }
        }
        if need_restructure {
            children = Node::create_parent_branches(&grandchildren);
        }

        Node::create_parent_branches(&children)
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
    pub fn new() -> Self {
        Leaf {
            chunk: String::new(),
        }
    }

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
