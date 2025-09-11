use std::{cmp, ops::Range, rc::Rc};
use unicode_segmentation::GraphemeCursor;

pub const MAX_CHUNK_SIZE: usize = if cfg!(test) { 16 } else { 128 };
pub const TREE_ORDER: usize = 16;

#[derive(Debug, Clone)]
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

    pub fn new_lines(&self) -> usize {
        match self {
            Self::Branch(branch) => branch.new_lines(),
            Self::Leaf(leaf) => leaf.new_lines(),
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
        Rc::clone(&Self::create_root(&nodes))
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

    pub fn slice(&self, range: Range<usize>) -> Rc<Node> {
        let nodes = self.slice_recursive(range);
        let root = Node::truncate_root(&nodes);
        Rc::clone(&root)
    }

    pub fn slice_recursive(&self, range: Range<usize>) -> Vec<Rc<Node>> {
        match self {
            Self::Branch(branch) => branch.slice(range),
            Self::Leaf(leaf) => leaf.slice(range),
        }
    }

    // create parent branch(es) for node(s)
    pub fn create_parent_branches(children: &[Rc<Node>]) -> Vec<Rc<Node>> {
        if children.is_empty() {
            return Vec::new();
        }

        let num_parents = children.len().div_ceil(TREE_ORDER);
        let parent_capacity = children.len().div_ceil(num_parents);
        let mut parents: Vec<Rc<Node>> = Vec::with_capacity(num_parents);

        for chunk in children.chunks(parent_capacity) {
            let branch_children = chunk.to_vec();
            let mut keys: Vec<usize> = Vec::new();
            let mut length: usize = 0;
            let mut new_lines: usize = 0;

            for child in chunk.iter().take(chunk.len().saturating_sub(1)) {
                length += child.len();
                keys.push(length);
                new_lines += child.new_lines();
            }

            if let Some(last_child) = chunk.last() {
                length += last_child.len();
                new_lines += last_child.new_lines();
            }

            parents.push(Rc::new(Node::Branch(Branch {
                new_lines,
                children: branch_children,
                height: children.first().unwrap().height() + 1,
                keys,
                length,
            })))
        }
        parents
    }

    // create parent branches until a root that support all provided branches is formed
    pub fn create_root(nodes: &[Rc<Node>]) -> Rc<Node> {
        let mut curr_nodes = nodes.to_vec();
        while curr_nodes.len() > 1 {
            curr_nodes = Node::create_parent_branches(&curr_nodes);
        }
        match curr_nodes.first() {
            None => Self::new(),
            Some(root) => Rc::clone(root),
        }
    }

    // remove nodes that are not necessary for the tree to have all of its data by traversing to the left
    // currently just used after deletion when it leaves a series of nodes from root to a certain nodes that each have a single child
    pub fn truncate_root(nodes: &[Rc<Node>]) -> Rc<Node> {
        let mut curr_nodes = nodes.to_vec();
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
        Self::new()
    }

    pub fn write_to(&self, buf: &mut String, range: Range<usize>) {
        match self {
            Self::Branch(branch) => {
                let targets = branch.find_children_by_range(range);
                let children = &branch.children;
                for target in targets {
                    children[target.0].write_to(buf, target.1);
                }
            }
            Self::Leaf(leaf) => buf.push_str(&leaf.as_str()[range]),
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
                "Leaves at inconsistent heights: min={min_height}, max={max_height}, found heights: {leaf_depths:?}"
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

#[derive(Debug, Clone)]
pub struct Branch {
    new_lines: usize,
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

    pub fn new_lines(&self) -> usize {
        self.new_lines
    }

    pub fn children(&self) -> &Vec<Rc<Node>> {
        &self.children
    }

    pub fn keys(&self) -> &Vec<usize> {
        &self.keys
    }

    // return the index of the child and the real index in the child
    pub fn find_child_by_index(&self, index: usize) -> (usize, usize) {
        match self.keys().binary_search(&index) {
            Ok(pos) => (pos + 1, index - self.keys()[pos]),
            Err(pos) => {
                let offset = if pos == 0 { 0 } else { self.keys()[pos - 1] };
                (pos, index - offset)
            }
        }
    }

    // return the indexes of the children and the real ranges in the them
    pub fn find_children_by_range(&self, range: Range<usize>) -> Vec<(usize, Range<usize>)> {
        if range.is_empty() {
            return Vec::new();
        }

        let start_child = match self.keys.binary_search(&range.start) {
            Ok(pos) => pos + 1,
            Err(pos) => pos,
        };

        let end_child = match self.keys.binary_search(&range.end) {
            Ok(pos) => pos + 1,
            Err(pos) => pos.min(self.children.len() - 1),
        };

        let mut result = Vec::with_capacity(end_child - start_child + 1);

        let mut offset = if start_child == 0 {
            0
        } else {
            self.keys[start_child - 1]
        };

        for i in start_child..=end_child {
            let child_end = if i < self.keys.len() {
                self.keys[i]
            } else {
                self.length
            };

            if range.start < child_end && offset < range.end {
                let real_range = (range.start.saturating_sub(offset))
                    ..(range.end.saturating_sub(offset).min(child_end - offset));
                result.push((i, real_range));
            }
            offset = child_end;
        }

        result
    }

    // recursively find the correct child to insert into and create new nodes while keeping unaffected nodes
    pub fn insert(&self, index: usize, text: &str) -> Vec<Rc<Node>> {
        let (insert_index, index_in_child) = self.find_child_by_index(index);
        let target_child = &self.children[insert_index];

        let new_children = target_child.insert_recursive(index_in_child, text);

        if new_children.len() == 1 && Rc::ptr_eq(&new_children[0], target_child) {
            return vec![Rc::new(Node::Branch(self.clone()))];
        }

        let mut children = Vec::with_capacity(self.children.len() - 1 + new_children.len());
        children.extend_from_slice(&self.children[..insert_index]);
        children.extend(new_children);
        children.extend_from_slice(&self.children[(insert_index + 1)..]);

        Node::create_parent_branches(&children)
    }

    // recursively find the correct children to delete and keep unaffected nodes
    pub fn delete(&self, range: Range<usize>) -> Vec<Rc<Node>> {
        let mut children = self.children.clone();

        let to_delete = self.find_children_by_range(range);

        if to_delete.is_empty() {
            return Node::create_parent_branches(&self.children);
        }

        let mut altered_children: Vec<Rc<Node>> = Vec::new();

        for (pos, range_in_child) in &to_delete {
            let to_alter = Rc::clone(&children[*pos]);
            let altered = to_alter.delete_recursive(range_in_child.clone());
            altered_children.extend(altered);
        }

        let start = to_delete.first().unwrap().0;
        let end = to_delete.last().unwrap().0;
        children.splice(start..=end, altered_children);

        if children.is_empty() {
            return Vec::new();
        }

        // No need to check if the children of the current branch is filled less than half its max capacity when children are leaves
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

    pub fn slice(&self, range: Range<usize>) -> Vec<Rc<Node>> {
        let to_include = self.find_children_by_range(range);
        let children = self.children.clone();
        let mut children_to_include = Vec::new();

        for (pos, range_in_child) in &to_include {
            let to_alter = Rc::clone(&children[*pos]);
            let altered = to_alter.slice_recursive(range_in_child.clone());
            children_to_include.extend(altered);
        }

        // No need to check if the children of the current branch is filled less than half its max capacity when children are leaves
        if children_to_include.first().unwrap().is_leaf() {
            return Node::create_parent_branches(&children_to_include);
        }

        let mut need_restructure = false;
        let mut grandchildren: Vec<Rc<Node>> = Vec::new();
        for child in &children_to_include {
            grandchildren.extend(child.children());
            if child.children().len() < TREE_ORDER / 2 {
                need_restructure = true;
            }
        }
        if need_restructure {
            children_to_include = Node::create_parent_branches(&grandchildren);
        }

        Node::create_parent_branches(&children_to_include)
    }
}

#[derive(Debug, Clone)]
pub struct Leaf {
    new_lines: usize,
    chunk: String,
}

impl From<&str> for Leaf {
    fn from(value: &str) -> Self {
        Leaf {
            new_lines: value.matches('\n').count(),
            chunk: value.to_owned(),
        }
    }
}

impl Leaf {
    pub fn new() -> Self {
        Leaf {
            new_lines: 0,
            chunk: String::new(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.chunk
    }

    pub fn len(&self) -> usize {
        self.chunk.len()
    }

    pub fn new_lines(&self) -> usize {
        self.new_lines
    }

    pub fn split_text_to_leaves(text: &str) -> Vec<Rc<Node>> {
        if text.is_empty() {
            return Vec::new();
        }

        let mut cursor = GraphemeCursor::new(0, text.len(), true);
        let num_chunks = text.len().div_ceil(MAX_CHUNK_SIZE);
        let chunk_size = text.len().div_ceil(num_chunks);
        let mut leaves: Vec<Rc<Node>> = Vec::with_capacity(num_chunks);

        while cursor.cur_cursor() < text.len() {
            let start = cursor.cur_cursor();
            cursor.set_cursor(cmp::min(start + chunk_size, text.len()));

            while !text.is_char_boundary(cursor.cur_cursor())
                || !cursor.is_boundary(text, 0).unwrap_or(false)
            {
                cursor.set_cursor(cursor.cur_cursor() + 1);
            }

            let end = cursor.cur_cursor();
            let chunk = &text[start..end];
            let new_leaf = Rc::new(Node::Leaf(Leaf::from(chunk)));
            leaves.push(new_leaf);
        }
        leaves
    }

    pub fn insert(&self, index: usize, text: &str) -> Vec<Rc<Node>> {
        let (before, after) = self.chunk.split_at(index);
        let mut new_text = String::with_capacity(self.len() + text.len());
        new_text.push_str(before);
        new_text.push_str(text);
        new_text.push_str(after);

        if new_text.len() <= MAX_CHUNK_SIZE {
            return vec![Rc::new(Node::Leaf(Leaf::from(new_text.as_str())))];
        }

        Self::split_text_to_leaves(&new_text)
    }

    pub fn delete(&self, range: Range<usize>) -> Vec<Rc<Node>> {
        let mut new_text = self.chunk.to_owned();
        new_text.replace_range(range, "");
        Self::split_text_to_leaves(&new_text)
    }

    pub fn slice(&self, range: Range<usize>) -> Vec<Rc<Node>> {
        let text = self.chunk.to_owned();
        Self::split_text_to_leaves(&text[range])
    }
}

#[cfg(test)]
mod test {
    // maybe write some test for this as well
}
