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
                chunk.buffer.len(),
                chunk.line_starts.len() - 1,
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

    pub fn is_empty(&self) -> bool {
        self.length == 0
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

    fn rightmost(&self, mut x: NodeRef) -> NodeRef {
        loop {
            let right_opt = { x.borrow().right.clone() };
            match right_opt {
                Some(r) => x = r,
                None => return x,
            }
        }
    }

    // Find node at document offset.
    // Returns (node, remainder within node.piece, node_start_offset)
    fn node_at(&self, mut offset: usize) -> Option<(NodeRef, usize, usize)> {
        let mut x_opt = self.root.clone();
        let mut node_start_offset = 0usize;

        while let Some(x) = x_opt {
            let (size_left, piece_len, left, right) = {
                let nb = x.borrow();
                (
                    nb.size_left,
                    nb.piece.length,
                    nb.left.clone(),
                    nb.right.clone(),
                )
            };

            if size_left > offset {
                x_opt = left;
            } else if size_left + piece_len >= offset {
                node_start_offset += size_left;
                let remainder = offset - size_left;
                return Some((x.clone(), remainder, node_start_offset));
            } else {
                offset -= size_left + piece_len;
                node_start_offset += size_left + piece_len;
                x_opt = right;
            }
        }
        None
    }

    // Convert a remainder within node.piece to its BufferCursor within the backing buffer
    fn position_in_buffer(&self, node: &NodeRef, remainder: usize) -> BufferCursor {
        let nb = node.borrow();
        let piece = &nb.piece;
        let buf_idx = piece.buffer_idx;
        let line_starts = &self.buffers[buf_idx].line_starts;

        let start_offset = line_starts[piece.start.line] + piece.start.column;
        let end_offset = line_starts[piece.end.line] + piece.end.column;
        let target = (start_offset + remainder).min(end_offset);

        let mut low = piece.start.line;
        let mut high = piece.end.line;
        let mut mid: usize = low;
        // binary search target in [low..=high]
        while low <= high {
            mid = (low + high) / 2;
            let mid_start = line_starts[mid];
            if mid == high {
                break;
            }
            let mid_stop = line_starts[mid + 1];
            if target < mid_start {
                if mid == 0 {
                    break;
                }
                high = mid - 1;
            } else if target >= mid_stop {
                low = mid + 1;
            } else {
                break;
            }
        }

        BufferCursor {
            line: mid,
            column: target - line_starts[mid],
        }
    }

    // Absolute offset in buffer for a given cursor
    fn offset_in_buffer(&self, buffer_idx: usize, cursor: BufferCursor) -> usize {
        let line_starts = &self.buffers[buffer_idx].line_starts;
        line_starts[cursor.line] + cursor.column
    }

    // Count line breaks between start and end cursors in a specific buffer (CR, LF, CRLF -> 1)
    fn get_line_feed_cnt(
        &self,
        buffer_idx: usize,
        start: BufferCursor,
        end: BufferCursor,
    ) -> usize {
        // mirror the TS logic:
        // If end.column == 0 => count complete lines between start.line and end.line
        if end.column == 0 {
            return end.line.saturating_sub(start.line);
        }

        let line_starts = &self.buffers[buffer_idx].line_starts;
        if end.line == line_starts.len() - 1 {
            // No \n after end
            return end.line.saturating_sub(start.line);
        }

        let next_line_start_offset = line_starts[end.line + 1];
        let end_offset = line_starts[end.line] + end.column;
        if next_line_start_offset > end_offset + 1 {
            // More than one character after end => cannot be '\n'
            return end.line.saturating_sub(start.line);
        }

        // next_line_start_offset == end_offset + 1 => character at end_offset is '\n'.
        // check previous char for '\r'
        let buffer = &self.buffers[buffer_idx].buffer;
        if end_offset > 0 && buffer.as_bytes()[end_offset - 1] == b'\r' {
            return end.line.saturating_sub(start.line) + 1;
        }
        end.line.saturating_sub(start.line)
    }

    // Build pieces for a given text. This baseline creates new backing buffers (not buffer 0)
    // to avoid cross-boundary CRLF complexities in the mutable change buffer.
    fn create_new_pieces(&mut self, mut text: &str) -> Vec<Piece> {
        const AVG_BUF: usize = 65535;
        let mut pieces: Vec<Piece> = Vec::new();

        while !text.is_empty() {
            // Initial desired size
            let max = text.len().min(AVG_BUF);

            // Find a safe UTF-8 boundary <= max
            let mut split = max;
            while split > 0 && !text.is_char_boundary(split) {
                split -= 1;
            }

            if split == 0 {
                // max fell inside the first char; take the first char fully (or entire text if single-char)
                split = text
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| i)
                    .unwrap_or(text.len());
            }

            // Avoid splitting a CRLF pair between chunks: if we are right after '\r' and next is '\n', include '\n'
            if split < text.len()
                && split > 0
                && text.as_bytes()[split - 1] == b'\r'
                && text.as_bytes()[split] == b'\n'
            {
                split += 1; // ASCII, still at UTF-8 boundary
            } else if split < text.len() && split > 0 && text.as_bytes()[split - 1] == b'\r' {
                // Optional: avoid ending a chunk with a dangling '\r'
                split -= 1; // ASCII, still a safe boundary
                // If that made split 0 and the text actually starts with CRLF, include both to keep them together
                if split == 0 && text.len() >= 2 && &text.as_bytes()[0..2] == b"\r\n" {
                    split = 2;
                }
            }

            let chunk = &text[..split];
            let line_starts = StringBuffer::create_line_starts(chunk);
            let buf_idx = self.buffers.len();
            self.buffers.push(StringBuffer {
                buffer: chunk.to_string(),
                line_starts: line_starts.clone(),
            });

            let end_line = line_starts.len() - 1;
            let end_col = chunk.len() - line_starts[end_line];
            let piece = Piece::new(
                buf_idx,
                BufferCursor::new(0, 0),
                BufferCursor::new(end_line, end_col),
                chunk.len(),                         // length in bytes
                line_starts.len().saturating_sub(1), // number of line breaks
            );
            pieces.push(piece);

            text = &text[split..];
        }

        pieces
    }

    fn rb_insert_left(&mut self, node: Option<NodeRef>, piece: Piece) -> Option<NodeRef> {
        let z = Rc::new(RefCell::new(TreeNode::new(piece)));
        if self.root.is_none() {
            z.borrow_mut().color = NodeColor::Black;
            self.root = Some(z.clone());
            return Some(z);
        }

        if let Some(parent_rc) = node {
            let mut parent_borrow = parent_rc.borrow_mut();
            if parent_borrow.left.is_none() {
                parent_borrow.left = Some(z.clone());
                drop(parent_borrow);
                z.borrow_mut().parent = Some(Rc::downgrade(&parent_rc));
            } else {
                let left_child = parent_borrow.left.clone().expect("left child existed");
                drop(parent_borrow);
                let prev = self.rightmost(left_child);
                {
                    let mut prev_b = prev.borrow_mut();
                    prev_b.right = Some(z.clone());
                }
                z.borrow_mut().parent = Some(Rc::downgrade(&prev));
            }
        } else {
            // If node is None but tree non-empty, insert to the left-most position.
            let mut x = self.root.clone().expect("root exists");
            loop {
                let left_opt = { x.borrow().left.clone() };
                match left_opt {
                    Some(l) => x = l,
                    None => {
                        {
                            let mut xb = x.borrow_mut();
                            xb.left = Some(z.clone());
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

    fn piece_from_range(&self, buffer_idx: usize, start: BufferCursor, end: BufferCursor) -> Piece {
        let start_off = self.offset_in_buffer(buffer_idx, start);
        let end_off = self.offset_in_buffer(buffer_idx, end);
        let length = end_off.saturating_sub(start_off);
        let lf = self.get_line_feed_cnt(buffer_idx, start, end);
        Piece::new(buffer_idx, start, end, length, lf)
    }

    fn delete_node_tail(&mut self, node: &NodeRef, new_end: BufferCursor) {
        let (buf, start) = {
            let nb = node.borrow();
            (nb.piece.buffer_idx, nb.piece.start)
        };
        let new_piece = self.piece_from_range(buf, start, new_end);
        {
            let mut nb = node.borrow_mut();
            nb.piece = new_piece;
        }
        self.recompute_tree_metadata(node.clone());
    }

    fn delete_node_head(&mut self, node: &NodeRef, new_start: BufferCursor) {
        let (buf, end) = {
            let nb = node.borrow();
            (nb.piece.buffer_idx, nb.piece.end)
        };
        let new_piece = self.piece_from_range(buf, new_start, end);
        {
            let mut nb = node.borrow_mut();
            nb.piece = new_piece;
        }
        self.recompute_tree_metadata(node.clone());
    }

    fn shrink_node(
        &mut self,
        node: &NodeRef,
        start: BufferCursor,
        end: BufferCursor,
    ) -> Option<NodeRef> {
        // node keeps left segment [piece.start, start)
        let (buf, old_start, old_end) = {
            let nb = node.borrow();
            (nb.piece.buffer_idx, nb.piece.start, nb.piece.end)
        };

        // Left piece
        let left_piece = self.piece_from_range(buf, old_start, start);
        {
            let mut nb = node.borrow_mut();
            nb.piece = left_piece;
        }
        self.recompute_tree_metadata(node.clone());

        // Right piece
        let right_piece = self.piece_from_range(buf, end, old_end);
        if right_piece.length > 0 {
            return self.rb_insert_right(Some(node.clone()), right_piece);
        }
        None
    }

    // Insert `value` at document offset `offset`
    pub fn insert(&mut self, mut offset: usize, value: &str) {
        if value.is_empty() {
            return;
        }

        // clamp
        if offset > self.length {
            offset = self.length;
        }

        let new_pieces = self.create_new_pieces(value);

        if self.root.is_none() {
            // Tree empty: insert all pieces to the right chain
            let mut last: Option<NodeRef> = None;
            for p in new_pieces {
                last = if let Some(prev) = last {
                    self.rb_insert_right(Some(prev), p)
                } else {
                    self.rb_insert_left(None, p)
                };
            }
            self.compute_buffer_metadata();
            return;
        }

        // Find target node
        let (node, remainder, node_start_offset) = match self.node_at(offset) {
            Some(t) => t,
            None => {
                // append at end
                let rightmost = self.root.clone().map(|r| self.rightmost(r)).unwrap();
                let mut last = Some(rightmost.clone());
                for p in new_pieces {
                    last = self.rb_insert_right(last, p);
                }
                self.compute_buffer_metadata();
                return;
            }
        };

        let piece_len = { node.borrow().piece.length };
        if node_start_offset == offset {
            // insert to the left of node
            // Insert pieces in order: last piece first to the left to maintain sequence
            let mut cur_left_of = Some(node.clone());
            for p in new_pieces.iter().rev() {
                cur_left_of = self.rb_insert_left(cur_left_of, p.clone());
            }
        } else if node_start_offset + piece_len > offset {
            // Insert in the middle: split node into left and right
            let split_pos = self.position_in_buffer(&node, remainder);

            // Right part from split_pos to old end
            let right_piece = {
                let nb = node.borrow();
                self.piece_from_range(nb.piece.buffer_idx, split_pos, nb.piece.end)
            };

            // Left part: truncate node tail to split_pos
            self.delete_node_tail(&node, split_pos);

            // Insert new pieces after node, then right piece after them
            let mut last = Some(node.clone());
            for p in new_pieces {
                last = self.rb_insert_right(last, p);
            }
            if right_piece.length > 0 {
                self.rb_insert_right(last, right_piece);
            }
        } else {
            // Insert to the right of this node
            let mut last = Some(node.clone());
            for p in new_pieces {
                last = self.rb_insert_right(last, p);
            }
        }

        self.compute_buffer_metadata();
    }

    // Delete `cnt` chars starting at `offset`
    pub fn delete(&mut self, offset: usize, mut cnt: usize) {
        if cnt == 0 || self.root.is_none() || offset >= self.length {
            return;
        }

        // clamp to end
        if offset + cnt > self.length {
            cnt = self.length - offset;
        }

        // Find start and end positions
        let (start_node, start_rem, start_node_start) = match self.node_at(offset) {
            Some(t) => t,
            None => return,
        };
        let end_offset = offset + cnt;
        let (end_node, end_rem, _end_node_start) = match self.node_at(end_offset) {
            Some(t) => t,
            None => {
                // End exactly at document end: walk to rightmost
                let last = self.root.clone().map(|r| self.rightmost(r)).unwrap();
                let last_len = { last.borrow().piece.length };
                (last, last_len, self.length - last_len)
            }
        };

        if Rc::ptr_eq(&start_node, &end_node) {
            // delete within one node
            let start_cursor = self.position_in_buffer(&start_node, start_rem);
            let end_cursor = self.position_in_buffer(&start_node, end_rem);

            if start_node_start == offset && cnt == start_node.borrow().piece.length {
                // delete entire node -> baseline: make it empty (no RB delete yet)
                let buf_idx = start_node.borrow().piece.buffer_idx;
                let empty_piece = self.piece_from_range(buf_idx, start_cursor, start_cursor);
                {
                    let mut nb = start_node.borrow_mut();
                    nb.piece = empty_piece;
                }
                self.recompute_tree_metadata(start_node.clone());
            } else if start_node_start == offset {
                // delete head
                self.delete_node_head(&start_node, end_cursor);
            } else if start_node_start + start_node.borrow().piece.length == end_offset {
                // delete tail
                self.delete_node_tail(&start_node, start_cursor);
            } else {
                // delete middle => shrink and insert right piece
                self.shrink_node(&start_node, start_cursor, end_cursor);
            }

            self.compute_buffer_metadata();
            return;
        }

        // Spanning multiple nodes:
        // 1) trim tail of start node
        let start_cursor = self.position_in_buffer(&start_node, start_rem);
        self.delete_node_tail(&start_node, start_cursor);

        // 2) zero out all nodes strictly between start_node and end_node
        let mut cur_opt = {
            // successor of start_node
            // If it has right child, successor is leftmost of right subtree
            // else climb up to first parent where we are in its left subtree
            let cur = start_node.clone();
            // use next()
            self.next(&cur)
        };
        while let Some(cur) = cur_opt.clone() {
            if Rc::ptr_eq(&cur, &end_node) {
                break;
            }
            // zero out piece
            let buf_idx = { cur.borrow().piece.buffer_idx };
            let zero =
                self.piece_from_range(buf_idx, BufferCursor::new(0, 0), BufferCursor::new(0, 0));
            {
                let mut nb = cur.borrow_mut();
                nb.piece = zero;
            }
            self.recompute_tree_metadata(cur.clone());

            cur_opt = self.next(&cur);
        }

        // 3) trim head of end node
        let end_cursor = self.position_in_buffer(&end_node, end_rem);
        // For end node, we need to delete head up to end_cursor
        let end_start_cursor = {
            let nb = end_node.borrow();
            nb.piece.start
        };
        self.delete_node_head(&end_node, end_cursor);

        self.compute_buffer_metadata();
    }

    // inorder successor
    fn next(&self, node: &NodeRef) -> Option<NodeRef> {
        if let Some(r) = { node.borrow().right.clone() } {
            return Some(self.leftmost(r));
        }
        // climb up
        let mut cur = node.clone();
        while let Some(p) = Self::parent_of(&cur) {
            let is_left = {
                let pb = p.borrow();
                if let Some(ref l) = pb.left {
                    Rc::ptr_eq(l, &cur)
                } else {
                    false
                }
            };
            if is_left {
                return Some(p);
            }
            cur = p;
        }
        None
    }

    // Compute accumulated byte length within a piece up to the given internal line index.
    // Mirrors TS getAccumulatedValue: if index < 0 => 0; if beyond piece end => piece length; else difference of line starts.
    fn get_accumulated_value(&self, node: &NodeRef, index: isize) -> usize {
        if index < 0 {
            return 0;
        }
        let nb = node.borrow();
        let piece = &nb.piece;
        let line_starts = &self.buffers[piece.buffer_idx].line_starts;
        let idx = index as usize;
        let expected_line_start_index = piece.start.line + idx + 1;
        if expected_line_start_index > piece.end.line {
            // up to end of piece
            return (line_starts[piece.end.line] + piece.end.column)
                .saturating_sub(line_starts[piece.start.line] + piece.start.column);
        } else {
            return line_starts[expected_line_start_index]
                .saturating_sub(line_starts[piece.start.line] + piece.start.column);
        }
    }

    // Given an accumulated byte count within a node's piece, return:
    // - index: how many line feeds are strictly before that position inside the piece
    // - remainder: byte remainder within the current (index-th) line
    fn get_index_of(&self, node: &NodeRef, accumulated_value: usize) -> (usize, usize) {
        let nb = node.borrow();
        let piece = &nb.piece;
        let buf_idx = piece.buffer_idx;

        let start_off = self.offset_in_buffer(buf_idx, piece.start);
        let end_off = self.offset_in_buffer(buf_idx, piece.end);

        let pos = self.position_in_buffer(node, accumulated_value);
        let line_cnt = pos.line.saturating_sub(piece.start.line);

        // If we're exactly at the end of the node, check CRLF boundary to adjust index
        if end_off.saturating_sub(start_off) == accumulated_value {
            let real_line_cnt = self.get_line_feed_cnt(buf_idx, piece.start, pos);
            if real_line_cnt != line_cnt {
                return (real_line_cnt, 0);
            }
        }

        (line_cnt, pos.column)
    }

    // 1-based (line, column) to 0-based offset in the whole document
    pub fn get_offset_at(&self, mut line_number: usize, column: usize) -> usize {
        if line_number == 0 {
            return 0;
        }

        let mut left_len: usize = 0;
        let mut x_opt = self.root.clone();

        while let Some(x) = x_opt {
            let (lf_left, size_left, piece_lf, piece_len, left, right) = {
                let nb = x.borrow();
                (
                    nb.lf_left,
                    nb.size_left,
                    nb.piece.line_feed_cnt,
                    nb.piece.length,
                    nb.left.clone(),
                    nb.right.clone(),
                )
            };

            // Go left if that subtree can cover the target line
            if left.is_some() && lf_left + 1 >= line_number {
                x_opt = left;
            } else if lf_left + piece_lf + 1 >= line_number {
                // Target line is inside this node's piece
                left_len += size_left;
                // line_number >= 2 here — do signed arithmetic to avoid usize underflow
                let idx = line_number as isize - lf_left as isize - 2;
                let acc = self.get_accumulated_value(&x, idx);
                return left_len + acc + column.saturating_sub(1);
            } else {
                // Skip this node and go right
                line_number = line_number.saturating_sub(lf_left + piece_lf);
                left_len += size_left + piece_len;
                x_opt = right;
            }
        }

        left_len
    }

    // 0-based offset to 1-based (line, column) document position
    pub fn get_position_at(&self, mut offset: usize) -> BufferCursor {
        let mut x_opt = self.root.clone();
        let mut lf_cnt: usize = 0;
        let original_offset = offset;

        while let Some(x) = x_opt {
            let (size_left, piece_len, lf_left, piece_lf, left, right) = {
                let nb = x.borrow();
                (
                    nb.size_left,
                    nb.piece.length,
                    nb.lf_left,
                    nb.piece.line_feed_cnt,
                    nb.left.clone(),
                    nb.right.clone(),
                )
            };

            if size_left != 0 && size_left >= offset {
                x_opt = left;
            } else if size_left + piece_len >= offset {
                let (index, remainder) = self.get_index_of(&x, offset - size_left);
                lf_cnt += lf_left + index;
                if index == 0 {
                    // Same line where node starts
                    let line_start_off = self.get_offset_at(lf_cnt + 1, 1);
                    let column0 = original_offset.saturating_sub(line_start_off);
                    return BufferCursor::new(lf_cnt + 1, column0 + 1);
                }
                return BufferCursor::new(lf_cnt + 1, remainder + 1);
            } else {
                offset = offset.saturating_sub(size_left + piece_len);
                lf_cnt += lf_left + piece_lf;
                if right.is_none() {
                    // last node
                    let line_start_off = self.get_offset_at(lf_cnt + 1, 1);
                    let column0 = original_offset
                        .saturating_sub(offset)
                        .saturating_sub(line_start_off);
                    return BufferCursor::new(lf_cnt + 1, column0 + 1);
                } else {
                    x_opt = right;
                }
            }
        }

        BufferCursor::new(1, 1)
    }

    // Get the display length of a line (without EOL)
    pub fn get_line_length(&self, line_number: usize) -> usize {
        self.get_line_content(line_number).len()
    }

    // Get the full document text by concatenating all pieces in-order
    pub fn get_text(&self) -> String {
        let mut out = String::new();
        self.for_each_inorder(|node| {
            let nb = node.borrow();
            let piece = &nb.piece;
            if piece.length == 0 {
                return true;
            }
            let buf_idx = piece.buffer_idx;
            if buf_idx >= self.buffers.len() {
                return true;
            }
            let buffer = &self.buffers[buf_idx].buffer;
            let line_starts = &self.buffers[buf_idx].line_starts;

            let start = line_starts[piece.start.line] + piece.start.column;
            let end = line_starts[piece.end.line] + piece.end.column;
            if start <= end && end <= buffer.len() {
                out.push_str(&buffer[start..end]);
            }
            true
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(tree: &PieceTree) -> String {
        tree.get_lines_content().join("\n")
    }

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

    #[test]
    fn insert_into_empty_and_append() {
        let mut chunks: Vec<StringBuffer> = vec![];
        let mut tree = PieceTree::new(chunks.as_mut_slice());

        // Insert into empty
        tree.insert(0, "Hello\nWorld");
        assert_eq!(tree.get_lines_content(), vec!["Hello", "World"]);

        // Insert in the middle (after "Hello")
        tree.insert(5, " Rust");
        assert_eq!(tree.get_lines_content(), vec!["Hello Rust", "World"]);

        // Insert at end
        let end = doc(&tree).len();
        tree.insert(end, "\n!!!");
        assert_eq!(tree.get_lines_content(), vec!["Hello Rust", "World", "!!!"]);
    }

    #[test]
    fn insert_begin_middle_end_positions() {
        let mut chunks: Vec<StringBuffer> = vec![];
        let mut tree = PieceTree::new(chunks.as_mut_slice());

        // Start with base
        tree.insert(0, "abc\ndef");
        assert_eq!(tree.get_lines_content(), vec!["abc", "def"]);

        // Insert at beginning
        tree.insert(0, ">>");
        assert_eq!(tree.get_lines_content(), vec![">>abc", "def"]);

        // Insert in the middle (between 'a' and 'b')
        tree.insert(3, "_MID_"); // positions: 0:>,1:>,2:a,3:b,...
        assert_eq!(tree.get_lines_content(), vec![">>a_MID_bc", "def"]);

        // Insert at end
        let end = doc(&tree).len();
        tree.insert(end, "\nEND");
        assert_eq!(tree.get_lines_content(), vec![">>a_MID_bc", "def", "END"]);
    }

    #[test]
    fn delete_within_single_node_middle() {
        let mut chunks: Vec<StringBuffer> = vec![];
        let mut tree = PieceTree::new(chunks.as_mut_slice());

        tree.insert(0, "Hello\nWorld");
        assert_eq!(tree.get_lines_content(), vec!["Hello", "World"]);

        // Delete "lo\nWo" starting at offset 3, length 5
        // "Hello\nWorld" indices: H0 e1 l2 l3 o4 \n5 W6 o7 r8 l9 d10
        tree.delete(3, 5);
        assert_eq!(doc(&tree), "Helrld");
        assert_eq!(tree.get_lines_content(), vec!["Helrld"]);
    }

    #[test]
    fn delete_spanning_multiple_nodes() {
        let mut chunks: Vec<StringBuffer> = vec![];
        let mut tree = PieceTree::new(chunks.as_mut_slice());

        // Build three separate nodes by separate inserts
        tree.insert(0, "foo\n");
        let end = doc(&tree).len();
        tree.insert(end, "bar\n");
        let end = doc(&tree).len();
        tree.insert(end, "baz");

        assert_eq!(doc(&tree), "foo\nbar\nbaz");
        assert_eq!(tree.get_lines_content(), vec!["foo", "bar", "baz"]);

        // Delete "o\nbar\n" which spans node boundaries
        // "foo\nbar\nbaz": f0 o1 o2 \n3 b4 a5 r6 \n7 b8 a9 z10
        // Delete from offset 2 length 6: indices [2..8)
        tree.delete(2, 6);
        assert_eq!(doc(&tree), "fobaz");
        assert_eq!(tree.get_lines_content(), vec!["fobaz"]);

        // Delete entire remaining content
        let total = doc(&tree).len();
        tree.delete(0, total);
        // An empty document is represented as a single empty line
        assert_eq!(tree.get_lines_content(), vec![""]);
    }

    #[test]
    fn delete_trailing_newline_boundary() {
        let mut chunks: Vec<StringBuffer> = vec![];
        let mut tree = PieceTree::new(chunks.as_mut_slice());

        tree.insert(0, "a\nb\n");
        assert_eq!(tree.get_lines_content(), vec!["a", "b", ""]);

        // Remove the last '\n'
        let total = doc(&tree).len(); // "a\nb\n" -> len = 4
        tree.delete(total - 1, 1);
        assert_eq!(tree.get_lines_content(), vec!["a", "b"]);

        // Now delete the middle newline
        // Current doc: "a\nb" -> indices: a0 \n1 b2
        tree.delete(1, 1);
        assert_eq!(tree.get_lines_content(), vec!["ab"]);
    }

    #[test]
    fn get_text_and_line_length() {
        let mut chunks: Vec<StringBuffer> = vec![];
        let mut tree = PieceTree::new(chunks.as_mut_slice());
        tree.insert(0, "abc\ndef");
        assert_eq!(tree.get_text(), "abc\ndef");
        assert_eq!(tree.get_line_length(1), 3);
        assert_eq!(tree.get_line_length(2), 3);
        assert_eq!(tree.get_line_length(3), 0);
    }

    #[test]
    fn offset_and_position_roundtrip() {
        let mut chunks: Vec<StringBuffer> = vec![];
        let mut tree = PieceTree::new(chunks.as_mut_slice());
        tree.insert(0, "012\n45\n789");

        // Offsets
        assert_eq!(tree.get_offset_at(1, 1), 0);
        assert_eq!(tree.get_offset_at(1, 4), 3);
        assert_eq!(tree.get_offset_at(2, 1), 4);
        assert_eq!(tree.get_offset_at(2, 3), 6);
        assert_eq!(tree.get_offset_at(3, 1), 7);
        assert_eq!(tree.get_offset_at(3, 4), 10);

        // Positions
        let p = tree.get_position_at(0);
        assert_eq!((p.line, p.column), (1, 1));
        let p = tree.get_position_at(3);
        assert_eq!((p.line, p.column), (1, 4));
        let p = tree.get_position_at(4);
        assert_eq!((p.line, p.column), (2, 1));
        let p = tree.get_position_at(6);
        assert_eq!((p.line, p.column), (2, 3));
        let p = tree.get_position_at(7);
        assert_eq!((p.line, p.column), (3, 1));
        let p = tree.get_position_at(10);
        assert_eq!((p.line, p.column), (3, 4));
    }

    #[test]
    fn utf8_safe_split_and_crlf_boundary() {
        // Pattern: multi-byte chars + CRLF
        let unit = "α😀β\r\n"; // α (2 bytes), 😀 (4 bytes), β (2 bytes), \r\n (2 bytes) => 10 bytes
        assert_eq!(unit.len(), 10);

        // Pad to make the initial split index (65535) land exactly after '\r' and before '\n'
        // 65535 % 10 = 5; we want 9 => add 6 extra bytes.
        let pad = "x".repeat(6); // ASCII, 6 bytes

        // Make the text longer than AVG_BUF (65535) so splitting occurs.
        // Use enough repeats to cross the boundary comfortably.
        let avg_buf = 65_535usize;
        let repeats = ((avg_buf - pad.len()) / unit.len()) + 2;

        // Build the full text: [pad][unit x repeats]
        let mut text = String::with_capacity(pad.len() + repeats * unit.len());
        text.push_str(&pad);
        for _ in 0..repeats {
            text.push_str(unit);
        }

        // Create an empty tree and insert the large text so create_new_pieces() is exercised.
        let mut chunks: Vec<StringBuffer> = vec![];
        let mut tree = PieceTree::new(chunks.as_mut_slice());
        tree.insert(0, &text);

        // Round-trip: ensure exact content is preserved across piece boundaries.
        assert_eq!(tree.get_text(), text);

        // Expected lines:
        // - First line: pad + "α😀β"
        // - Next `repeats-1` lines: "α😀β"
        // - Trailing empty line because the text ends with CRLF
        let mut expected_lines: Vec<String> = Vec::with_capacity(repeats + 1);
        expected_lines.push(format!("{}{}", pad, "α😀β"));
        for _ in 1..repeats {
            expected_lines.push("α😀β".to_string());
        }
        expected_lines.push(String::new());
        assert_eq!(tree.get_lines_content(), expected_lines);

        // Sanity checks on offsets/positions around line boundaries.

        // Start of line 1.
        assert_eq!(tree.get_offset_at(1, 1), 0);

        // End of line 1 length (in bytes) equals pad.len() + "α😀β".len()
        let line1_len = tree.get_line_length(1);
        assert_eq!(line1_len, pad.len() + "α😀β".len());

        // Start of line 2 offset should be end-of-line1 + CRLF length (2 bytes).
        let eol_len = 2; // the source text uses CRLF
        let offset_line2 = line1_len + eol_len;
        assert_eq!(tree.get_offset_at(2, 1), offset_line2);
        assert_eq!(tree.get_position_at(offset_line2).line, 2);

        // Verify the last (trailing) line is empty.
        assert_eq!(tree.get_line_length(repeats + 1), 0);
    }
}
