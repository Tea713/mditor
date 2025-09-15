use std::cell::RefCell;
use std::rc::{Rc, Weak};

type NodeRef = Rc<RefCell<TreeNode>>;
type WeakNodeRef = Weak<RefCell<TreeNode>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferCursor {
    line: usize,
    column: usize,
}

impl BufferCursor {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Clone)]
pub struct Piece {
    buffer_idx: usize,
    start: BufferCursor,
    end: BufferCursor,
    length: usize,
    line_feed_cnt: usize,
}

impl Piece {
    pub fn new(
        buffer_idx: usize,
        start: BufferCursor,
        end: BufferCursor,
        length: usize,
        line_feed_cnt: usize,
    ) -> Self {
        Self {
            buffer_idx,
            start,
            end,
            length,
            line_feed_cnt,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StringBuffer {
    buffer: String,
    line_starts: Vec<usize>,
}

impl StringBuffer {
    pub fn new(buffer: String) -> Self {
        let line_starts = Self::create_line_starts(&buffer);
        Self {
            buffer,
            line_starts,
        }
    }

    pub fn create_line_starts(text: &str) -> Vec<usize> {
        let mut line_starts = vec![0];
        let bytes = text.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let chr = bytes[i];

            match chr {
                b'\r' => {
                    if i + 1 < len && bytes[i + 1] == b'\n' {
                        // \r\n case
                        line_starts.push(i + 2);
                        i += 1; // skip the \n
                    } else {
                        // \r case
                        line_starts.push(i + 1);
                    }
                }
                b'\n' => {
                    // \n case - Unix line ending
                    line_starts.push(i + 1);
                }
                _ => {}
            }

            i += 1;
        }

        line_starts
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeColor {
    Red,
    Black,
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    piece: Piece,
    color: NodeColor,
    parent: Option<WeakNodeRef>,
    left: Option<NodeRef>,
    right: Option<NodeRef>,
    size_left: usize,
    lf_left: usize,
}

impl TreeNode {
    pub fn new(piece: Piece) -> Self {
        Self {
            piece,
            color: NodeColor::Red,
            parent: None,
            left: None,
            right: None,
            size_left: 0,
            lf_left: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PieceTree {
    root: Option<NodeRef>,
    buffers: Vec<StringBuffer>,
    length: usize,
    line_count: usize,
    eol: &'static str,
}

impl PieceTree {
    pub fn new(chunks: &mut [StringBuffer]) -> Self {
        let mut tree = Self {
            root: None,
            buffers: vec![StringBuffer::new(String::new())],
            line_count: 1,
            length: 0,
            eol: "\n",
        };

        if chunks.is_empty() {
            return tree;
        };

        let mut last_node: Option<NodeRef> = None;
        for (i, chunk) in chunks.iter().enumerate() {
            let piece = Piece::new(
                i + 1,
                BufferCursor::new(0, 0),
                BufferCursor::new(
                    chunk.line_starts.len() - 1,
                    chunk.buffer.len() - chunk.line_starts[chunk.line_starts.len() - 1],
                ),
                chunk.line_starts.len() - 1,
                chunk.buffer.len(),
            );
            tree.buffers.push(chunk.clone());
            last_node = tree.rb_insert_right(last_node, piece);
        }

        tree.compute_buffer_metadata();
        tree
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn line_count(&self) -> usize {
        self.line_count
    }

    fn for_each_inorder<F: FnMut(&NodeRef) -> bool>(&self, mut f: F) {
        let mut stack: Vec<NodeRef> = Vec::new();
        let mut cur = self.root.clone();

        while cur.is_some() || !stack.is_empty() {
            while let Some(c) = cur {
                let left = { c.borrow().left.clone() };
                stack.push(c);
                cur = left;
            }

            let node = stack.pop().unwrap();
            if !f(&node) {
                break;
            }
            cur = node.borrow().right.clone();
        }
    }

    fn char_code_at(s: &str, idx: usize) -> Option<u8> {
        s.as_bytes().get(idx).copied()
    }

    // Strip a single trailing EOL sequence from [start, end)
    // Returns the new end index that excludes the EOL
    fn strip_trailing_eol_range(s: &str, start: usize, end: usize) -> usize {
        if end <= start {
            return end;
        }
        // handle ...\r\n
        if end >= start + 2 {
            let a = s.as_bytes()[end - 2];
            let b = s.as_bytes()[end - 1];
            if a == b'\r' && b == b'\n' {
                return end - 2;
            }
        }
        // handle ...\n or ...\r
        let last = s.as_bytes()[end - 1];
        if last == b'\n' || last == b'\r' {
            return end - 1;
        }
        end
    }

    pub fn get_lines_content(&self) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let mut current_line = String::new();
        let mut dangling_cr = false;

        self.for_each_inorder(|node| {
            let nb = node.borrow();
            let piece = &nb.piece;

            // Resolve buffer and ranges
            let buf_idx = piece.buffer_idx;
            if buf_idx >= self.buffers.len() {
                // Skip invalid piece
                return true;
            }
            let buffer = &self.buffers[buf_idx].buffer;
            let line_starts = &self.buffers[buf_idx].line_starts;

            // Compute absolute offsets
            let piece_start_line = piece.start.line;
            let piece_end_line = piece.end.line;
            if piece_start_line >= line_starts.len() || piece_end_line >= line_starts.len() {
                return true;
            }
            let mut piece_start_offset = line_starts[piece_start_line] + piece.start.column;
            let piece_end_offset = line_starts[piece_end_line] + piece.end.column;

            if piece_end_offset < piece_start_offset || piece_start_offset > buffer.len() {
                return true;
            }
            let mut piece_length = piece_end_offset.saturating_sub(piece_start_offset);
            if piece_length == 0 {
                return true;
            }

            // Handle dangling CR across piece boundary
            if dangling_cr {
                if let Some(b'\n') = Self::char_code_at(buffer, piece_start_offset) {
                    // pretend the \n was in the previous piece
                    piece_start_offset += 1;
                    piece_length = piece_length.saturating_sub(1);
                }
                // close previous line
                lines.push(std::mem::take(&mut current_line));
                dangling_cr = false;

                if piece_length == 0 {
                    return true;
                }
            }

            if piece_start_line == piece_end_line {
                // No newlines fully inside this piece segment
                let end = piece_start_offset + piece_length;
                if piece_length > 0 && Self::char_code_at(buffer, end - 1) == Some(b'\r') {
                    // Leave CR dangling to the next piece if next char is LF
                    dangling_cr = true;
                    if piece_start_offset < end - 1 {
                        current_line.push_str(&buffer[piece_start_offset..end - 1]);
                    }
                } else {
                    current_line.push_str(&buffer[piece_start_offset..end]);
                }
                return true;
            }

            // Add the text before the first line start in this piece
            // End at next line start (includes that line's EOL); we strip the EOL.
            let first_line_next_start = line_starts[piece_start_line + 1];
            let mut seg_end = first_line_next_start.min(piece_end_offset);
            seg_end = Self::strip_trailing_eol_range(buffer, piece_start_offset, seg_end);
            if piece_start_offset < seg_end {
                current_line.push_str(&buffer[piece_start_offset..seg_end]);
            }
            lines.push(std::mem::take(&mut current_line));

            // Emit intermediate full lines inside the piece
            for line in (piece_start_line + 1)..piece_end_line {
                let start = line_starts[line];
                let mut end = line_starts[line + 1];
                end = end.min(buffer.len());
                let trimmed_end = Self::strip_trailing_eol_range(buffer, start, end);
                current_line.clear();
                if start < trimmed_end {
                    current_line.push_str(&buffer[start..trimmed_end]);
                }
                lines.push(std::mem::take(&mut current_line));
            }

            // Handle the last (partial) line segment of the piece
            let end_line_start = line_starts[piece_end_line];
            let end_abs = piece_end_offset;

            if piece.end.column == 0 {
                // The piece ends exactly at the start of a line. If the character
                // before this line is '\r', mark dangling and undo previous push.
                if end_line_start > 0
                    && Self::char_code_at(buffer, end_line_start - 1) == Some(b'\r')
                {
                    dangling_cr = true;
                    if !lines.is_empty() {
                        lines.pop();
                    }
                    current_line.clear();
                } else {
                    current_line.clear();
                }
            } else {
                // Take substring of the ending line up to the column
                // If the last char is '\r', mark dangling and exclude it
                if end_abs > 0 && Self::char_code_at(buffer, end_abs - 1) == Some(b'\r') {
                    dangling_cr = true;
                    current_line.clear();
                    if end_line_start < end_abs - 1 {
                        current_line.push_str(&buffer[end_line_start..end_abs - 1]);
                    }
                } else {
                    current_line.clear();
                    if end_line_start < end_abs {
                        current_line.push_str(&buffer[end_line_start..end_abs]);
                    }
                }
            }

            true
        });

        if dangling_cr {
            // finalize the dangling CR line
            lines.push(std::mem::take(&mut current_line));
        }

        // push the remaining current line (last line)
        lines.push(current_line);
        lines
    }

    pub fn get_line_content(&self, line_number: usize) -> String {
        let lines = self.get_lines_content();
        if line_number == 0 {
            return String::new();
        }
        if line_number <= lines.len() {
            return lines[line_number - 1].clone();
        }
        String::new()
    }

    fn parent_of(node: &NodeRef) -> Option<NodeRef> {
        node.borrow().parent.as_ref().and_then(|w| w.upgrade())
    }

    fn is_left_child_of_parent(&self, node: &NodeRef) -> Option<bool> {
        let parent = Self::parent_of(node)?;
        let pb = parent.borrow();
        if let Some(ref l) = pb.left {
            if Rc::ptr_eq(l, node) {
                return Some(true);
            }
        }
        if let Some(ref r) = pb.right {
            if Rc::ptr_eq(r, node) {
                return Some(false);
            }
        }
        None
    }

    fn set_parent(child: &NodeRef, parent: Option<&NodeRef>) {
        child.borrow_mut().parent = parent.map(|p| Rc::downgrade(p));
    }

    fn node_color(node: Option<&NodeRef>) -> NodeColor {
        match node {
            None => NodeColor::Black,
            Some(n) => n.borrow().color,
        }
    }

    fn set_color(node: &NodeRef, color: NodeColor) {
        node.borrow_mut().color = color;
    }

    fn left_of(node: &NodeRef) -> Option<NodeRef> {
        node.borrow().left.clone()
    }
    fn right_of(node: &NodeRef) -> Option<NodeRef> {
        node.borrow().right.clone()
    }

    fn leftmost(&self, mut x: NodeRef) -> NodeRef {
        loop {
            let left_opt = { x.borrow().left.clone() };
            match left_opt {
                Some(left) => {
                    x = left;
                }
                None => return x,
            }
        }
    }

    fn rb_insert_right(&mut self, node: Option<NodeRef>, piece: Piece) -> Option<NodeRef> {
        let z = Rc::new(RefCell::new(TreeNode::new(piece)));

        if self.root.is_none() {
            // Tree is empty: z becomes root and is black
            z.borrow_mut().color = NodeColor::Black;
            self.root = Some(z.clone());
            return Some(z);
        }

        if let Some(parent_rc) = node {
            // given a node; attach to its right if empty,
            // otherwise go to left-most node in node.right and attach as its left
            let mut parent_borrow = parent_rc.borrow_mut();
            if parent_borrow.right.is_none() {
                parent_borrow.right = Some(z.clone());
                drop(parent_borrow); // release before mutating z
                z.borrow_mut().parent = Some(Rc::downgrade(&parent_rc));
            } else {
                let right_child = parent_borrow.right.clone().expect("right child existed");
                drop(parent_borrow); // release before traversing
                let next = self.leftmost(right_child);
                {
                    let mut next_borrow = next.borrow_mut();
                    next_borrow.left = Some(z.clone());
                }
                z.borrow_mut().parent = Some(Rc::downgrade(&next));
            }
        } else {
            // If node is None but the tree is non-empty, we can interpret this as:
            // insert to the right-most position of the current tree.
            // This path won't be used in your current new(), but it's safe to define.
            let mut x = self.root.clone().expect("root exists");
            loop {
                let right_opt = { x.borrow().right.clone() };
                match right_opt {
                    Some(r) => x = r,
                    None => {
                        {
                            let mut xb = x.borrow_mut();
                            xb.right = Some(z.clone());
                        }
                        z.borrow_mut().parent = Some(Rc::downgrade(&x));
                        break;
                    }
                }
            }
        }

        self.fix_insert(z.clone());
        Some(z)
    }

    fn subtree_size(node: Option<NodeRef>) -> usize {
        match node {
            None => 0,
            Some(rc) => {
                let nb = rc.borrow();
                let left = nb.left.clone();
                let right = nb.right.clone();
                Self::subtree_size(left) + nb.piece.length + Self::subtree_size(right)
            }
        }
    }

    fn subtree_lf(node: Option<NodeRef>) -> usize {
        match node {
            None => 0,
            Some(rc) => {
                let nb = rc.borrow();
                let left = nb.left.clone();
                let right = nb.right.clone();
                Self::subtree_lf(left) + nb.piece.line_feed_cnt + Self::subtree_lf(right)
            }
        }
    }

    fn left_rotate(&mut self, x: NodeRef) {
        let y_opt = { x.borrow().right.clone() };
        let y = match y_opt {
            None => return, // nothing to rotate
            Some(n) => n,
        };

        // Cache values needed for metadata update
        let (x_size_left, x_lf_left, x_piece_len, x_piece_lf) = {
            let xb = x.borrow();
            (
                xb.size_left,
                xb.lf_left,
                xb.piece.length,
                xb.piece.line_feed_cnt,
            )
        };

        // y.size_left += x.size_left + x.piece.length;
        // y.lf_left += x.lf_left + x.piece.lineFeedCnt;
        {
            let mut yb = y.borrow_mut();
            yb.size_left = yb.size_left.saturating_add(x_size_left + x_piece_len);
            yb.lf_left = yb.lf_left.saturating_add(x_lf_left + x_piece_lf);
        }

        // x.right = y.left
        let y_left = { y.borrow().left.clone() };
        {
            let mut xb = x.borrow_mut();
            xb.right = y_left.clone();
        }
        if let Some(ref yl) = y_left {
            Self::set_parent(yl, Some(&x));
        }

        // y.parent = x.parent; attach y to x.parent
        let x_parent = Self::parent_of(&x);
        Self::set_parent(&y, x_parent.as_ref());
        match x_parent {
            None => {
                // x was root
                self.root = Some(y.clone());
            }
            Some(p) => {
                let is_left = {
                    let pb = p.borrow();
                    if let Some(ref l) = pb.left {
                        Rc::ptr_eq(l, &x)
                    } else {
                        false
                    }
                };
                let mut pb = p.borrow_mut();
                if is_left {
                    pb.left = Some(y.clone());
                } else {
                    pb.right = Some(y.clone());
                }
            }
        }

        // y.left = x
        {
            let mut yb = y.borrow_mut();
            yb.left = Some(x.clone());
        }
        // x.parent = y
        Self::set_parent(&x, Some(&y));

        // Optionally recompute up the tree (safe and simple)
        self.recompute_tree_metadata(y);
    }

    fn right_rotate(&mut self, y: NodeRef) {
        let x_opt = { y.borrow().left.clone() };
        let x = match x_opt {
            None => return, // nothing to rotate
            Some(n) => n,
        };

        // Cache values needed for metadata update
        let (x_size_left, x_lf_left, x_piece_len, x_piece_lf) = {
            let xb = x.borrow();
            (
                xb.size_left,
                xb.lf_left,
                xb.piece.length,
                xb.piece.line_feed_cnt,
            )
        };

        // y.left = x.right
        let x_right = { x.borrow().right.clone() };
        {
            let mut yb = y.borrow_mut();
            yb.left = x_right.clone();
        }
        if let Some(ref xr) = x_right {
            Self::set_parent(xr, Some(&y));
        }

        // x.parent = y.parent
        let y_parent = Self::parent_of(&y);
        Self::set_parent(&x, y_parent.as_ref());
        match y_parent {
            None => {
                // y was root
                self.root = Some(x.clone());
            }
            Some(p) => {
                let is_right = {
                    let pb = p.borrow();
                    if let Some(ref r) = pb.right {
                        Rc::ptr_eq(r, &y)
                    } else {
                        false
                    }
                };
                let mut pb = p.borrow_mut();
                if is_right {
                    pb.right = Some(x.clone());
                } else {
                    pb.left = Some(x.clone());
                }
            }
        }

        // fix size_left on y: y.size_left -= x.size_left + x.piece.length
        // fix lf_left on y:   y.lf_left -= x.lf_left + x.piece.lineFeedCnt
        {
            let mut yb = y.borrow_mut();
            let sub = x_size_left + x_piece_len;
            let lf_sub = x_lf_left + x_piece_lf;
            yb.size_left = yb.size_left.saturating_sub(sub);
            yb.lf_left = yb.lf_left.saturating_sub(lf_sub);
        }

        // x.right = y
        {
            let mut xb = x.borrow_mut();
            xb.right = Some(y.clone());
        }
        // y.parent = x
        Self::set_parent(&y, Some(&x));

        self.recompute_tree_metadata(x);
    }

    // ---------- Insert fix-up (RB insert balancing) ----------
    fn fix_insert(&mut self, mut x: NodeRef) {
        // First, recompute metadata from x upwards
        self.recompute_tree_metadata(x.clone());

        while let Some(parent) = Self::parent_of(&x) {
            if Self::node_color(Some(&parent)) != NodeColor::Red {
                break;
            }
            // Safe to unwrap grandparent because parent is red (can't be root if root is black invariant)
            let grand = match Self::parent_of(&parent) {
                None => break,
                Some(g) => g,
            };

            let parent_is_left = {
                let gb = grand.borrow();
                if let Some(ref l) = gb.left {
                    Rc::ptr_eq(l, &parent)
                } else {
                    false
                }
            };

            if parent_is_left {
                let uncle = { grand.borrow().right.clone() };
                if Self::node_color(uncle.as_ref()) == NodeColor::Red {
                    // Case 1
                    Self::set_color(&parent, NodeColor::Black);
                    if let Some(ref u) = uncle {
                        Self::set_color(u, NodeColor::Black);
                    }
                    Self::set_color(&grand, NodeColor::Red);
                    x = grand.clone();
                } else {
                    // Case 2/3
                    // If x is right child, rotate left at parent
                    let x_is_right = {
                        let pb = parent.borrow();
                        if let Some(ref r) = pb.right {
                            Rc::ptr_eq(r, &x)
                        } else {
                            false
                        }
                    };
                    if x_is_right {
                        x = parent.clone();
                        self.left_rotate(x.clone());
                    }
                    // Case 3
                    let parent2 = Self::parent_of(&x).expect("parent after rotate");
                    let grand2 = Self::parent_of(&parent2).expect("grandparent after rotate");
                    Self::set_color(&parent2, NodeColor::Black);
                    Self::set_color(&grand2, NodeColor::Red);
                    self.right_rotate(grand2);
                }
            } else {
                // Mirror cases
                let uncle = { grand.borrow().left.clone() };
                if Self::node_color(uncle.as_ref()) == NodeColor::Red {
                    // Case 1
                    Self::set_color(&parent, NodeColor::Black);
                    if let Some(ref u) = uncle {
                        Self::set_color(u, NodeColor::Black);
                    }
                    Self::set_color(&grand, NodeColor::Red);
                    x = grand.clone();
                } else {
                    // Case 2/3
                    let x_is_left = {
                        let pb = parent.borrow();
                        if let Some(ref l) = pb.left {
                            Rc::ptr_eq(l, &x)
                        } else {
                            false
                        }
                    };
                    if x_is_left {
                        x = parent.clone();
                        self.right_rotate(x.clone());
                    }
                    let parent2 = Self::parent_of(&x).expect("parent after rotate");
                    let grand2 = Self::parent_of(&parent2).expect("grandparent after rotate");
                    Self::set_color(&parent2, NodeColor::Black);
                    Self::set_color(&grand2, NodeColor::Red);
                    self.left_rotate(grand2);
                }
            }
        }

        if let Some(ref root) = self.root {
            Self::set_color(root, NodeColor::Black);
            // root has no parent
            root.borrow_mut().parent = None;
        }

        // Recompute metadata for the entire path up from x to root
        self.recompute_tree_metadata(x);
    }

    fn compute_buffer_metadata(&mut self) {
        let mut x = self.root.clone();

        let mut lf_cnt = 1;
        let mut len = 0;

        while let Some(node) = x {
            let node_ref = node.borrow();
            lf_cnt += node_ref.lf_left + node_ref.piece.line_feed_cnt;
            len += node_ref.size_left + node_ref.piece.length;
            x = node_ref.right.clone();
        }

        self.line_count = lf_cnt;
        self.length = len;
    }

    fn recompute_tree_metadata(&mut self, mut x: NodeRef) {
        // Recompute size_left and lf_left for x and all its ancestors
        let mut cur: Option<NodeRef> = Some(x.clone());
        while let Some(n) = cur {
            let left = { n.borrow().left.clone() };
            let new_size_left = Self::subtree_size(left.clone());
            let new_lf_left = Self::subtree_lf(left);
            {
                let mut nb = n.borrow_mut();
                nb.size_left = new_size_left;
                nb.lf_left = new_lf_left;
            }
            cur = Self::parent_of(&n);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_basic_operations() {
    //     let mut tree = PieceTree::new("Hello\nWorld");
    //     assert_eq!(tree.get_length(), 11);
    //     assert_eq!(tree.get_line_count(), 2);

    //     tree.insert(5, " Rust");
    //     assert_eq!(tree.get_text(), "Hello Rust\nWorld");

    //     assert_eq!(tree.get_line(1), "Hello Rust");
    //     assert_eq!(tree.get_line(2), "World");
    // }

    #[test]
    fn lines_basic_unix() {
        let mut chunks = vec![StringBuffer::new("Hello\nWorld".to_string())];
        let tree = PieceTree::new(chunks.as_mut_slice());

        let lines = tree.get_lines_content();
        assert_eq!(lines, vec!["Hello", "World"]);

        assert_eq!(tree.get_line_content(1), "Hello");
        assert_eq!(tree.get_line_content(2), "World");
        // Out of range returns empty
        assert_eq!(tree.get_line_content(3), "");
    }

    #[test]
    fn lines_crlf_single_buffer() {
        // Contains Windows-style CRLF newlines
        let mut chunks = vec![StringBuffer::new("abc\r\ndef\r\nxyz".to_string())];
        let tree = PieceTree::new(chunks.as_mut_slice());

        let lines = tree.get_lines_content();
        assert_eq!(lines, vec!["abc", "def", "xyz"]);

        assert_eq!(tree.get_line_content(1), "abc");
        assert_eq!(tree.get_line_content(2), "def");
        assert_eq!(tree.get_line_content(3), "xyz");
        assert_eq!(tree.get_line_content(4), "");
    }

    #[test]
    fn lines_multiple_chunks() {
        // Split across pieces without CR/LF boundary complications
        let mut chunks = vec![
            StringBuffer::new("foo\n".to_string()),
            StringBuffer::new("bar\nbaz".to_string()),
        ];
        let tree = PieceTree::new(chunks.as_mut_slice());

        let lines = tree.get_lines_content();
        assert_eq!(lines, vec!["foo", "bar", "baz"]);

        assert_eq!(tree.get_line_content(1), "foo");
        assert_eq!(tree.get_line_content(2), "bar");
        assert_eq!(tree.get_line_content(3), "baz");
        assert_eq!(tree.get_line_content(4), "");
    }

    #[test]
    fn lines_trailing_newline() {
        // Ensure trailing newline yields final empty line
        let mut chunks = vec![StringBuffer::new("a\nb\n".to_string())];
        let tree = PieceTree::new(chunks.as_mut_slice());

        let lines = tree.get_lines_content();
        assert_eq!(lines, vec!["a", "b", ""]);

        assert_eq!(tree.get_line_content(1), "a");
        assert_eq!(tree.get_line_content(2), "b");
        assert_eq!(tree.get_line_content(3), "");
        assert_eq!(tree.get_line_content(4), "");
    }
}
