#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave14/agent04.rs
// ===========================================================================
// Wave 14 translation batch 04 (staging).
//
// Window: `0x00676000..0x00677fff`.
//
// The window is the middle of the `blz` C++ runtime linked into the 1.23.10
// client: the MSVC `std::_Tree` red-black engine as instantiated for the
// shipped string-keyed ordered containers, one intrusive `std::list` node
// helper, the interned-string release path, and the
// `blz::_shared_ptr_manager` deleting destructor.  None of these are game
// rules; they are the container layer the client's configuration and name
// tables are built on.
//
// ## Recovered layout
//
// The engine keeps a *header* node that doubles as the `end()` sentinel, so
// every link word is a node pointer and `isnil` is a byte flag.  The batches
// model the header by `None`.
//
// | location | shape | meaning |
// |---|---|---|
// | node `+0x00` / `+0x04` / `+0x08` | 3 pointers | left / parent / right |
// | node `+0x0c` | byte | colour, `1` = black (the header is `0x0101`) |
// | node `+0x0d` | byte | `_Isnil`, `1` = the header sentinel |
// | node `+0x10` | 4 words | value block, see below |
// | tree object `+0x00` / `+0x04` | pointer / `usize` | header pointer / node count |
// | list node `+0x00` / `+0x04` / `+0x08` | 2 pointers + 3 words | next / previous / value block |
// | intern table `+0x14` / `+0x18` | pointer / `usize` | list header / interned string count |
// | teardown object `+0x24` / `+0x28` / `+0x2c` | pointer / `usize` / `i32` | `0x1c`-byte record array, count and release flag |
// | teardown object `+0x04` / `+0x08` | 2 x `i32` | reference counts of the shared object |
// | teardown object `+0x10` / `+0x24` | `i32` / pointer | release flags of the two owned blocks |
//
// The value block has two shipped shapes.  A `set` leaf copies four words
// (a `0x10`-byte string with its inline buffer, `0x00676990`), while a
// `map` node copies three words and copy-constructs a nested tree into
// `+0x1c` (`0x006769c0`).  The batch keeps the four words verbatim and
// carries the ordering key as a decoded byte slice, because the comparator
// only ever looks through the string pointer.
//
// `_Buynode` at `679210` initialises a fresh `0x20`-byte node to
// `{self, self, self}` with the word `0x0101` at `+0x0c`, i.e. a black,
// `isnil` header; that is why an empty insert terminates immediately.
//
// ## Skipped rows
//
// * `006760b5` - one-byte `return`.
// * `006760c0`, `00676127`, `0067615f`, `00676190` - MSVC SEH
//   construct/destruct fragments: a virtual call through `in_EAX`, locals
//   addressed through `unaff_EBP`/`unaff_FS_OFFSET`, and a tail into the
//   non-returning throw helpers `00c86b6e`/`0093ec70`.  `00676190` also
//   reads `local_34`, which Ghidra never assigns.
// * `00676320`, `00676360` - tail jumps into `00748d40` with the caller's
//   SEH frame still installed; no recovered arguments.
// * `006765f7` - misaligned block: an `unaff_retaddr` store, a write at
//   `-0x3b7c2bbb` and a `CONCAT31` byte rotate of an argument.
// * `0067697d`, `00676a3f`, `00676b2f`, `00676ece`, `00677081` - `Catch_All`
//   landing pads: one free helper call fed from an `unaff_EBP` slot, then
//   `00aad4a0(0, 0)`.
// * `00676ca9` - `swi(3)` trap fragment (`int3` padding).
// * `00676ea3` - interior block of `00676ef0`: calls the unresolved
//   `676b50` with no arguments and copies the result through `unaff_EBP`.
// * `00676ef0` - `insert(hint, value)` adjustment.  The hint branch splits
//   `piVar9`/`piVar8` across the predecessor search that the decompile
//   allocates to disjoint blocks, and the duplicate-key guard reaches an
//   unrecovered abort; only the descent half is reliable, and that half is
//   already translated at `0x00676e10`.
// * `006770e0`, `0067722f`, `006773f0` - mid-function string tails: each
//   writes the empty SSO header (`+0x00` buffer, `+0x04` size `0`,
//   `+0x08` capacity `0x8000000f`) and then enters a non-returning helper.
// * `00677650`, `006776b0`, `00677700` - destructor tails: `_Tidy` of the
//   `0x24`/`0x20` instantiation followed by `operator delete(header, size)`.
//   The structural half is translated at `0x00677600`.
// * `00677940`, `00677977` - the two-level `map<string, set<string>>` insert.
//   The first row's decompile mixes three inlined iterator paths with the
//   erase-and-reinsert branch, and the second row starts at `+0x37` inside
//   the same body with every operand in `unaff_EBP`; neither yields a
//   consistent traversal.
// * `00677ca0` - the cached pseudocode stops at `00c89580`, which Ghidra
//   marks non-returning.  The shipped body continues for another `0x3c`
//   bytes (allocator copy of the string, list push at `+0x14`), so the
//   cached prefix is not the whole routine.
// * `00677db0` - erase range: four blocks were removed as unreachable
//   (`677f1b..677f92`) inside the erase loop, and the loop's node pointer is
//   reloaded from a stale `local_18[0]`.
// * `00677fb0` - profile/INI parser.  It dispatches through the character
//   class helpers `00678cc0`, `00678ce0`, `00678d10`, `00678d30` and
//   `00678f70` in the next window, and the section-name loop has an
//   unrecovered `extraout_AL`.
//
// Every routine below is a detached reference helper: nothing is wired into
// `World`, `SimUnit`, the renderer or the client, and no save/replay state
// is introduced.

// ---------------------------------------------------------------------------
// Recovered sizes and identities
// ---------------------------------------------------------------------------

/// Size of one `_Tree` node of the string-keyed instantiation (`_Buynode` at
/// `679210` allocates exactly this many bytes).
pub const W14A04_NODE_BYTES: usize = 0x20;

/// Size of one intrusive list node (`_Buynode` at `679250`; `0x00677d50`
/// releases it with the same size class).
pub const W14A04_LIST_NODE_BYTES: usize = 0x14;

/// Size of one record in the array destroyed by `0x00677750`.
pub const W14A04_RECORD_BYTES: u32 = 0x1c;

/// First node count rejected by `0x00676cb0`; the largest accepted count is
/// therefore `0x71c71c5`.  The sibling `_Insert` at `676b50` rejects
/// `0x7fffffe` and up instead.
pub const W14A04_MAX_NODES: usize = 0x71c_71c6;

/// `blz::_shared_ptr_manager` vtable stored by `0x00677910`
/// (`0x00d3287c`, the same identity the wave 13 destructor at `0065c140`
/// installs).
pub const W14A04_SHARED_PTR_MANAGER_VTABLE: u32 = 0x00d3_287c;

// ---------------------------------------------------------------------------
// Recovered node / tree shapes
// ---------------------------------------------------------------------------

/// One `_Tree` node, reconstructed from the shipped `0x20`-byte layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14A04Node<'a> {
    /// `+0x00`: left child; `None` is the header sentinel.
    pub left: Option<u32>,
    /// `+0x04`: parent; `None` is the header sentinel.
    pub parent: Option<u32>,
    /// `+0x08`: right child; `None` is the header sentinel.
    pub right: Option<u32>,
    /// `+0x0c`: colour byte, `true` for the shipped `1` (black).
    pub black: bool,
    /// `+0x0d`: the `_Isnil` byte.
    pub nil: bool,
    /// `+0x10..=+0x1c`: the value block words.  For the `set` instantiation
    /// these are the string pointer, size, capacity word and inline buffer
    /// word; for the `map` instantiation the fourth word is the nested tree.
    pub words: [u32; 4],
    /// The ordering key the comparator reads through the `+0x10` pointer.
    /// The shipped node stores a `std::string`; the helper carries the
    /// decoded NUL-terminated bytes instead of the pointer, so an empty slice
    /// is the empty string (not a null pointer).
    pub key: &'a [u8],
}

/// One `_Tree` container object: the header links plus the node count that
/// `0x00676cb0` and `0x00677600` update at container offset `+0x04`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14A04Tree {
    /// Header `+0x04`: root node.
    pub root: Option<u32>,
    /// Header `+0x00`: leftmost node (`begin()`).
    pub leftmost: Option<u32>,
    /// Header `+0x08`: rightmost node.
    pub rightmost: Option<u32>,
    /// Container `+0x04`: node count.
    pub size: usize,
}

/// Failure states a retail write would turn into a fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W14A04TreeError {
    /// The node count reached [`W14A04_MAX_NODES`] and the shipped code
    /// aborts with `"map/set<T> too long"`.
    TooLong,
    /// A link index is outside the modelled arena; the retail dereference
    /// would fault.
    MissingNode(u32),
}

// ---------------------------------------------------------------------------
// 0x006762a0 / 0x006762b0 - constant predicate and boolean text
// ---------------------------------------------------------------------------

/// `0x006762a0`: the five-byte constant predicate.
///
/// ```asm
/// b0 01        mov al, 1
/// c3           ret
/// ```
///
/// The routine has no arguments and no reads; it always answers `1`.
pub const fn w14a04_always_true() -> bool {
    true
}

/// `0x006762b0`: the boolean-to-text helper, `param_2 ? "true" : "false"`.
///
/// The shipped body selects the literal, measures it with the inlined
/// `strlen` loop, builds an SSO `std::string` (`+0x00` buffer, `+0x04` size
/// `0`, `+0x08` capacity `0x8000000f`, first buffer byte `0`) and hands the
/// length to the tail helper `00538810`.  Only the selected literal is
/// modelled; the string object is the shipped string ABI.
pub fn w14a04_bool_text(flag: bool) -> &'static str {
    if flag { "true" } else { "false" }
}

// ---------------------------------------------------------------------------
// 0x00677890 / 0x00677830 - ordering and iteration
// ---------------------------------------------------------------------------

/// `0x00677890`: case-insensitive "less" over NUL-terminated strings.
///
/// The routine folds `'A'..='Z'` to lower case (the shipped byte test is
/// `(c - 0x41) <= 0x19`, so only that ASCII range folds), compares the two
/// characters as **signed** bytes (`movsbl` before `sub`), and finally
/// reports whether `right` still has bytes left:
///
/// ```c
/// while (*a != 0) {
///   if (*b == 0) return 0;                 // b is a prefix of a
///   if (fold(*a) != fold(*b)) return fold(*a) < fold(*b);
///   a++; b++;
/// }
/// return *b != 0;                         // a is a proper prefix of b
/// ```
///
/// The comparator is the `_Tree` key predicate of every ordered container in
/// this window; callers invert it to get `>=`.
pub fn w14a04_ci_less(left: &[u8], right: &[u8]) -> bool {
    let mut index = 0usize;
    loop {
        let current = left.get(index).copied().unwrap_or(0);
        if current == 0 {
            return right.get(index).copied().unwrap_or(0) != 0;
        }
        let other = right.get(index).copied().unwrap_or(0);
        if other == 0 {
            return false;
        }
        let folded_left = w14a04_ci_fold(current);
        let folded_right = w14a04_ci_fold(other);
        if folded_left != folded_right {
            return folded_left < folded_right;
        }
        index += 1;
    }
}

/// The `'A'..='Z'` lowering step shared by `0x00677890`.
pub fn w14a04_ci_fold(byte: u8) -> i8 {
    if (0x41..=0x5a).contains(&byte) {
        (byte + 0x20) as i8
    } else {
        byte as i8
    }
}

/// `0x006770a0`: `_Tree::lower_bound` (the `_Lbound` descent).
///
/// ```c
/// result = header;                       // end()
/// for (node = root; !node->isnil; ) {
///   if (FUN_00677890(key, node->key)) node = node->right;
///   else { result = node; node = node->left; }
/// }
/// return result;                         // header means "not found"
/// ```
///
/// Returns the first node whose key is not less than `key`, or `None` for the
/// header sentinel (`end()`).
pub fn w14a04_lower_bound<'a>(
    root: Option<u32>,
    nodes: &[W14A04Node<'a>],
    key: &[u8],
) -> Option<u32> {
    let mut result = None;
    let mut cursor = root;
    while let Some(index) = cursor {
        let Some(node) = nodes.get(index as usize) else {
            break;
        };
        if w14a04_ci_less(node.key, key) {
            cursor = node.right;
        } else {
            result = Some(index);
            cursor = node.left;
        }
    }
    result
}

/// `0x00677830`: `_Tree::const_iterator::operator++`.
///
/// ```c
/// if (!node->right->isnil) {          // descend to the leftmost of the right
///   for (n = node->right; !n->left->isnil; n = n->left) {}
///   return n;
/// }
/// for (parent = node->parent; !parent->isnil && node == parent->right; ) {
///   node = parent; parent = parent->parent;
/// }
/// return parent;                      // header means end()
/// ```
///
/// `None` is the header, so an increment from the rightmost node yields
/// `None`.
pub fn w14a04_next<'a>(nodes: &[W14A04Node<'a>], index: u32) -> Option<u32> {
    let node = nodes.get(index as usize)?;
    if let Some(right) = node.right {
        let mut cursor = right;
        while let Some(left) = nodes.get(cursor as usize).and_then(|next| next.left) {
            cursor = left;
        }
        Some(cursor)
    } else {
        let mut current = index;
        let mut parent = node.parent;
        while let Some(up) = parent {
            if nodes.get(up as usize).and_then(|link| link.right) != Some(current) {
                break;
            }
            current = up;
            parent = nodes.get(up as usize).and_then(|link| link.parent);
        }
        parent
    }
}

/// Leftmost node of `root` (the descent both `0x006768b0` and `0x00677830`
/// use), `None` for an empty tree.
pub fn w14a04_leftmost<'a>(nodes: &[W14A04Node<'a>], root: Option<u32>) -> Option<u32> {
    let mut cursor = root;
    while let Some(left) = nodes.get(cursor? as usize).and_then(|node| node.left) {
        cursor = Some(left);
    }
    cursor
}

/// Rightmost node of `root`, `None` for an empty tree.
pub fn w14a04_rightmost<'a>(nodes: &[W14A04Node<'a>], root: Option<u32>) -> Option<u32> {
    let mut cursor = root;
    while let Some(right) = nodes.get(cursor? as usize).and_then(|node| node.right) {
        cursor = Some(right);
    }
    cursor
}

// ---------------------------------------------------------------------------
// 0x00676990 / 0x006769c0 / 0x00676a60 - node constructors
// ---------------------------------------------------------------------------

/// `0x00676990`: `_Buynode(value)` for the four-word value block.
///
/// ```c
/// node = _Buynode(0x20);     // 679280
/// node->word_c = 0;          // colour and _Isnil cleared together
/// node->w[0] = src[0]; node->w[1] = src[1];
/// node->w[2] = src[2]; node->w[3] = src[3];
/// return node;
/// ```
///
/// The three link words are left untouched here; `_Insert` (`0x00676cb0`) and
/// `_Copy_nodes` (`0x00676a90`) fill them in.
pub fn w14a04_node_new<'a>(words: [u32; 4]) -> W14A04Node<'a> {
    W14A04Node {
        left: None,
        parent: None,
        right: None,
        black: false,
        nil: false,
        words,
        key: &[],
    }
}

/// A node whose fourth value word is the nested tree object of the `map`
/// instantiation, kept as a separate arena because an index pair cannot be
/// stored in one word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W14A04TwoLevelNode<'a> {
    /// The freshly constructed outer node.
    pub node: W14A04Node<'a>,
    /// Copy of the nested tree header.
    pub nested: W14A04Tree,
    /// Copy of the nested tree nodes.
    pub nested_nodes: Vec<W14A04Node<'a>>,
}

/// `0x006769c0`: `_Buynode` for the two-level `map` value.
///
/// ```c
/// node = _Buynode(0x24);      // 6792a0
/// node->word_c = 0;
/// node->w[0] = src[0]; node->w[1] = src[1]; node->w[2] = src[2];
/// _Tree(node + 0x1c, src + 0x0c);   // copy-construct 0x006768b0
/// return node;
/// ```
///
/// The nested copy runs the same `0x006768b0` constructor that the clone
/// helper below models, so the fourth word is not written here.
pub fn w14a04_node_new_with_tree<'a>(
    words: [u32; 3],
    key: &'a [u8],
    nested: &W14A04Tree,
    nested_nodes: &[W14A04Node<'a>],
) -> W14A04TwoLevelNode<'a> {
    let (tree, nodes) = w14a04_tree_clone(nested, nested_nodes);
    W14A04TwoLevelNode {
        node: W14A04Node {
            left: None,
            parent: None,
            right: None,
            black: false,
            nil: false,
            words: [words[0], words[1], words[2], 0],
            key,
        },
        nested: tree,
        nested_nodes: nodes,
    }
}

/// One intrusive list node, the `0x14`-byte shape `0x00676a60` builds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14A04SmallNode {
    /// `+0x00`: next link (`None` = list header sentinel).
    pub next: Option<u32>,
    /// `+0x04`: previous link (`None` = list header sentinel).
    pub prev: Option<u32>,
    /// `+0x08..=+0x10`: the three-word value block.
    pub words: [u32; 3],
}

/// `0x00676a60`: the intrusive list `_Buynode(next, prev)` plus value copy.
///
/// ```c
/// node = _Buynode(next, prev);   // 679250, 0x14 bytes
/// node->v[0] = src[0];
/// node->v[1] = src[1];
/// node->v[2] = src[2];
/// return node;
/// ```
///
/// The two links are taken as supplied; `_Buynode` substitutes the node
/// itself for a null `next`, which the caller reads back through the return
/// value.
pub fn w14a04_small_node_new(
    next: Option<u32>,
    prev: Option<u32>,
    words: [u32; 3],
) -> W14A04SmallNode {
    W14A04SmallNode { next, prev, words }
}

// ---------------------------------------------------------------------------
// 0x00676a90 / 0x006768b0 - recursive copy and copy constructor
// ---------------------------------------------------------------------------

/// `0x00676a90`: `_Copy_nodes`, the recursive subtree copy.
///
/// ```c
/// _Newroot = *this;                        // the new tree's header
/// if (src->isnil) return _Newroot;
/// node = _Buynode(src + 0x10);             // 0x00676990
/// node->parent = parent;
/// node->color  = src->color;               // _Isnil is not copied
/// if (_Newroot->isnil) _Newroot = node;
/// node->left  = _Copy_nodes(src->left,  node, ...);
/// node->right = _Copy_nodes(src->right, node, ...);
/// return _Newroot;
/// ```
///
/// Nodes are appended in pre-order, so a parent's index always precedes its
/// children's.  A missing source index returns `None`, the header sentinel.
pub fn w14a04_copy_nodes<'a>(
    source: &[W14A04Node<'a>],
    node: Option<u32>,
    parent: Option<u32>,
    out: &mut Vec<W14A04Node<'a>>,
) -> Option<u32> {
    let index = node?;
    let source_node = *source.get(index as usize)?;
    let new_index = out.len() as u32;
    out.push(W14A04Node {
        left: None,
        parent,
        right: None,
        black: source_node.black,
        nil: false,
        words: source_node.words,
        key: source_node.key,
    });
    let left = w14a04_copy_nodes(source, source_node.left, Some(new_index), out);
    out[new_index as usize].left = left;
    let right = w14a04_copy_nodes(source, source_node.right, Some(new_index), out);
    out[new_index as usize].right = right;
    Some(new_index)
}

/// `0x006768b0`: `_Tree(const _Tree&)`, the container copy constructor.
///
/// ```c
/// this->header = 0; this->size = 0;
/// this->header = _Buynode();                     // 679210, black nil header
/// this->header->parent = _Copy_nodes(src->root, this->header);
/// this->size = src->size;
/// if (this->header->parent->isnil)
///   this->header->left = this->header;           // empty: begin() = end()
/// else
///   this->header->left = leftmost(this->header->parent);
/// this->header->right = rightmost(this->header->parent);
/// ```
///
/// The leftmost descent starts at the copied root and stops one level above
/// the sentinel; the rightmost descent is the mirror image.
pub fn w14a04_tree_clone<'a>(
    source: &W14A04Tree,
    source_nodes: &[W14A04Node<'a>],
) -> (W14A04Tree, Vec<W14A04Node<'a>>) {
    let mut nodes = Vec::new();
    let root = w14a04_copy_nodes(source_nodes, source.root, None, &mut nodes);
    let tree = W14A04Tree {
        root,
        leftmost: w14a04_leftmost(&nodes, root),
        rightmost: w14a04_rightmost(&nodes, root),
        size: source.size,
    };
    (tree, nodes)
}

// ---------------------------------------------------------------------------
// 0x00676cb0 / 0x00676e10 - insertion and the red-black fix-up
// ---------------------------------------------------------------------------

/// `0x00676cb0`: `_Tree::_Insert`, the link plus red-black fix-up.
///
/// ```c
/// if (this->size >= 0x71c71c6) throw "map/set<T> too long";
/// this->size++;
/// node->parent = where;
/// if (where == header) { header->parent = node; header->left = node;
///                        header->right = node; }
/// else if (!addleft)   { where->right = node;
///                        if (where == header->right) header->right = node; }
/// else                 { where->left = node;
///                        if (where == header->left)  header->left = node; }
/// for (x = node; ; ) {
///   if (parent(x)->color) { root->color = 1; return node; }
///   grand = parent(x)->parent;
///   if (parent(x) == grand->left) {
///     uncle = grand->right;
///     if (!uncle->color) { parent black; uncle black; grand red; x = grand; }
///     else {
///       if (x == parent(x)->right) { _Lrotate(parent(x)); x = parent(x); }
///       parent(x) black; grand red; _Rrotate(grand);
///     }
///   } else { /* mirror image; the last rotation is inlined */ }
/// }
/// ```
///
/// The shipped fix-up calls `FUN_006796a0` for a left rotation, `FUN_00679700`
/// for a right rotation, and inlines the final left rotation of the mirrored
/// branch.  Both directions are modelled by the two helpers below.
pub fn w14a04_insert<'a>(
    tree: &mut W14A04Tree,
    nodes: &mut [W14A04Node<'a>],
    add_left: bool,
    parent: Option<u32>,
    new_node: u32,
) -> Result<u32, W14A04TreeError> {
    if tree.size >= W14A04_MAX_NODES {
        return Err(W14A04TreeError::TooLong);
    }
    if new_node as usize >= nodes.len() {
        return Err(W14A04TreeError::MissingNode(new_node));
    }
    if let Some(index) = parent {
        if index as usize >= nodes.len() {
            return Err(W14A04TreeError::MissingNode(index));
        }
    }
    tree.size += 1;
    nodes[new_node as usize].parent = parent;
    match parent {
        None => {
            tree.root = Some(new_node);
            tree.leftmost = Some(new_node);
            tree.rightmost = Some(new_node);
        }
        Some(where_) if add_left => {
            nodes[where_ as usize].left = Some(new_node);
            if tree.leftmost == Some(where_) {
                tree.leftmost = Some(new_node);
            }
        }
        Some(where_) => {
            nodes[where_ as usize].right = Some(new_node);
            if tree.rightmost == Some(where_) {
                tree.rightmost = Some(new_node);
            }
        }
    }

    let mut current = new_node;
    loop {
        let Some(link) = nodes.get(current as usize).copied() else {
            break;
        };
        let Some(up) = link.parent else {
            break;
        };
        if nodes[up as usize].black {
            break;
        }
        let Some(grand) = nodes[up as usize].parent else {
            // A red node always has a black grandparent in a well-formed
            // tree; stop instead of reading the header.
            break;
        };
        if nodes[grand as usize].left == Some(up) {
            let uncle = nodes[grand as usize].right;
            if uncle.is_some_and(|index| !nodes[index as usize].black) {
                let uncle = uncle.unwrap_or(up);
                nodes[up as usize].black = true;
                nodes[uncle as usize].black = true;
                nodes[grand as usize].black = false;
                current = grand;
            } else {
                if nodes[up as usize].right == Some(current) {
                    w14a04_rotate_left(nodes, tree, up);
                    current = up;
                }
                let red = nodes[current as usize].parent.unwrap_or(up);
                nodes[red as usize].black = true;
                let top = nodes[red as usize].parent.unwrap_or(grand);
                nodes[top as usize].black = false;
                w14a04_rotate_right(nodes, tree, top);
            }
        } else {
            let uncle = nodes[grand as usize].left;
            if uncle.is_some_and(|index| !nodes[index as usize].black) {
                let uncle = uncle.unwrap_or(up);
                nodes[up as usize].black = true;
                nodes[uncle as usize].black = true;
                nodes[grand as usize].black = false;
                current = grand;
            } else {
                if nodes[up as usize].left == Some(current) {
                    w14a04_rotate_right(nodes, tree, up);
                    current = up;
                }
                let red = nodes[current as usize].parent.unwrap_or(up);
                nodes[red as usize].black = true;
                let top = nodes[red as usize].parent.unwrap_or(grand);
                nodes[top as usize].black = false;
                w14a04_rotate_left(nodes, tree, top);
            }
        }
    }
    if let Some(root) = tree.root {
        nodes[root as usize].black = true;
    }
    Ok(new_node)
}

/// `FUN_006796a0`: the left rotation used by `0x00676cb0` and inlined as the
/// final rotation of the mirrored branch.
///
/// ```c
/// up = pivot->right;
/// pivot->right = up->left;
/// if (!up->left->isnil) up->left->parent = pivot;
/// up->parent = pivot->parent;
/// if (pivot == header->parent) header->parent = up;
/// else if (pivot == pivot->parent->left) pivot->parent->left  = up;
/// else                                   pivot->parent->right = up;
/// up->left = pivot;
/// pivot->parent = up;
/// ```
///
/// No header link is touched beyond the root: a rotation never changes which
/// node is the leftmost or the rightmost.
pub fn w14a04_rotate_left<'a>(nodes: &mut [W14A04Node<'a>], tree: &mut W14A04Tree, pivot: u32) {
    let Some(up) = nodes.get(pivot as usize).and_then(|node| node.right) else {
        return;
    };
    if up as usize >= nodes.len() {
        return;
    }
    let parent = nodes[pivot as usize].parent;
    let moved = nodes[up as usize].left;
    nodes[pivot as usize].right = moved;
    if let Some(index) = moved {
        nodes[index as usize].parent = Some(pivot);
    }
    nodes[up as usize].parent = parent;
    if tree.root == Some(pivot) {
        tree.root = Some(up);
    } else if let Some(index) = parent {
        if nodes[index as usize].left == Some(pivot) {
            nodes[index as usize].left = Some(up);
        } else {
            nodes[index as usize].right = Some(up);
        }
    }
    nodes[up as usize].left = Some(pivot);
    nodes[pivot as usize].parent = Some(up);
}

/// `FUN_00679700`: the right rotation used by `0x00676cb0`, mirror of
/// [`w14a04_rotate_left`].
pub fn w14a04_rotate_right<'a>(nodes: &mut [W14A04Node<'a>], tree: &mut W14A04Tree, pivot: u32) {
    let Some(up) = nodes.get(pivot as usize).and_then(|node| node.left) else {
        return;
    };
    if up as usize >= nodes.len() {
        return;
    }
    let parent = nodes[pivot as usize].parent;
    let moved = nodes[up as usize].right;
    nodes[pivot as usize].left = moved;
    if let Some(index) = moved {
        nodes[index as usize].parent = Some(pivot);
    }
    nodes[up as usize].parent = parent;
    if tree.root == Some(pivot) {
        tree.root = Some(up);
    } else if let Some(index) = parent {
        if nodes[index as usize].left == Some(pivot) {
            nodes[index as usize].left = Some(up);
        } else {
            nodes[index as usize].right = Some(up);
        }
    }
    nodes[up as usize].right = Some(pivot);
    nodes[pivot as usize].parent = Some(up);
}

/// `0x00676e10`: the leaf-descent unique insert, returning
/// `pair<iterator, bool>`.
///
/// ```c
/// where = header; addleft = 1;
/// for (node = header->parent; !node->isnil; ) {
///   where = node;
///   addleft = addleft_arg ? !less(node->key, key) : less(key, node->key);
///   node = addleft ? node->left : node->right;
/// }
/// pair = _Insert(addleft, where, key, newnode);   // 676b50
/// return (pair.first, 1);
/// ```
///
/// The descent always reaches a leaf, so the header special case of `_Insert`
/// is only used for the first node.  The routine does **not** test for a
/// duplicate key: the shipped duplicate guard lives in `00676ef0`.
pub fn w14a04_insert_at<'a>(
    tree: &mut W14A04Tree,
    nodes: &mut [W14A04Node<'a>],
    add_left: bool,
    key: &[u8],
    new_node: u32,
) -> Result<(u32, bool), W14A04TreeError> {
    let mut where_node = None;
    let mut insert_left = true;
    let mut cursor = tree.root;
    while let Some(index) = cursor {
        let Some(node) = nodes.get(index as usize) else {
            break;
        };
        where_node = Some(index);
        let descend_left = if add_left {
            !w14a04_ci_less(node.key, key)
        } else {
            w14a04_ci_less(key, node.key)
        };
        insert_left = descend_left;
        cursor = if descend_left { node.left } else { node.right };
    }
    let inserted = w14a04_insert(tree, nodes, insert_left, where_node, new_node)?;
    Ok((inserted, true))
}

// ---------------------------------------------------------------------------
// 0x00677600 / 0x006775f0 - tidy and owned-block release
// ---------------------------------------------------------------------------

/// `0x00677600`: `_Tree::_Tidy`, the structural clear.
///
/// ```c
/// header = this->header;
/// first  = header->left;          // old begin(), the free-chain head
/// header->left   = header;
/// header->parent = header;
/// this->size     = 0;
/// while (first != header) { next = first->left; free(first, 0x14); first = next; }
/// free(header, 0x14);
/// ```
///
/// Three details are preserved: the count is cleared, the free walk reuses the
/// **left** link as its chain, and `header->right` (the rightmost pointer) is
/// left untouched because the header is released anyway.
pub fn w14a04_tidy(tree: &mut W14A04Tree) {
    tree.leftmost = None;
    tree.root = None;
    tree.size = 0;
}

/// `0x006775f0`: release the `0x20`-byte block owned by the container.
///
/// ```asm
/// 6a 20        push 0x20
/// ff 31        push dword ptr [ecx]
/// e8 ...       call 00923e90
/// c3           ret
/// ```
///
/// Returns the `(pointer, size)` pair the shipped code hands to the
/// allocator; the pool bookkeeping lives in `00923e90`.
pub fn w14a04_release_owned_block(owner: u32) -> (u32, usize) {
    (owner, W14A04_NODE_BYTES)
}

// ---------------------------------------------------------------------------
// 0x00677750 - container teardown with reference-count release
// ---------------------------------------------------------------------------

/// `0x00677910`: the `blz::_shared_ptr_manager` scalar deleting destructor.
///
/// ```c
/// *this = blz::_shared_ptr_manager::vftable;   // 0x00d3287c
/// if (flags & 1) operator delete(this);
/// return this;
/// ```
///
/// Ghidra lost the delete size operand; the vtable identity and the
/// `flags & 1` predicate are the recovered parts.  Returns whether the
/// deleting branch runs.
pub fn w14a04_shared_ptr_manager_dtor(manager: &mut W14A04SharedPtrManager, flags: u8) -> bool {
    manager.vtable = W14A04_SHARED_PTR_MANAGER_VTABLE;
    flags & 1 != 0
}

/// The single recovered field of `blz::_shared_ptr_manager` in this window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14A04SharedPtrManager {
    /// `+0x00`: vtable address.
    pub vtable: u32,
}

/// Reference-count release computed by `0x00677750` for the object at `+0x04`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14A04RefCountRelease {
    /// The `+0x04` counter reached zero, so `vtable[2]` (`+0x08` slot) runs.
    pub first_zero: bool,
    /// The `+0x08` counter left after the chained release, when it ran.
    pub second_after: Option<i32>,
    /// The `+0x08` counter reached zero, so `vtable[3]` (`+0x0c` slot) runs.
    pub second_zero: bool,
}

/// Teardown plan computed by `0x00677750`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14A04TeardownPlan {
    /// `+0x24`/`+0x28`: the record array as `(begin, end)`, stride `0x1c`.
    pub records: (u32, u32),
    /// Number of records passed to the range destroyer `00539c80`.
    pub records_len: u32,
    /// The `+0x2c` flag is non-negative, so the array block is released.
    pub array_released: bool,
    /// The `+0x10` flag is non-negative, so the `+0x08` block is released.
    pub second_released: bool,
    /// Release sequence of the reference-counted object at `+0x04`.
    pub refs: Option<W14A04RefCountRelease>,
}

/// `0x00677750`: container destructor.
///
/// ```c
/// destroy_range(+0x24, +0x24 + +0x28 * 0x1c);    // 00539c80
/// if (+0x2c >= 0) { alloc->vtable[4](+0x24); +0x24 = 0; }
/// if (+0x10 >= 0) { alloc->vtable[4](+0x08); }
/// if (object = +0x04) {
///   if (--object->refs1 == 0) {
///     object->vtable[2]();
///     if (--object->refs2 == 0) object->vtable[3]();
///   }
/// }
/// ```
///
/// The two block releases and the record destroy are modelled as flags and a
/// byte range; the counters are modelled by [`w14a04_release_refcounted`].
pub fn w14a04_teardown_plan(
    array: (u32, u32),
    array_flag: i32,
    second_flag: i32,
    object: Option<(i32, i32)>,
) -> W14A04TeardownPlan {
    let (begin, count) = array;
    W14A04TeardownPlan {
        records: (
            begin,
            begin.wrapping_add(count.wrapping_mul(W14A04_RECORD_BYTES)),
        ),
        records_len: count,
        array_released: array_flag >= 0,
        second_released: second_flag >= 0,
        refs: object.map(|(first, second)| w14a04_release_refcounted(first, second)),
    }
}

/// The `lock xadd` release pair at the end of `0x00677750`.
///
/// The first counter is decremented unconditionally; the second only when the
/// first reached zero, because the `vtable[2]` release runs in between.
pub fn w14a04_release_refcounted(first: i32, second: i32) -> W14A04RefCountRelease {
    let first_zero = first.wrapping_sub(1) == 0;
    let second_after = if first_zero {
        Some(second.wrapping_sub(1))
    } else {
        None
    };
    W14A04RefCountRelease {
        first_zero,
        second_after,
        second_zero: second_after == Some(0),
    }
}

// ---------------------------------------------------------------------------
// 0x00677d50 - release of an interned string
// ---------------------------------------------------------------------------

/// One node of the interned-string list released by `0x00677d50`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14A04PoolBlock {
    /// `+0x00`: next link (`None` = list header sentinel).
    pub next: Option<u32>,
    /// `+0x04`: previous link (`None` = list header sentinel).
    pub prev: Option<u32>,
    /// `+0x08`: the interned string pointer this node owns.
    pub text: u32,
}

/// Unlink plan produced by `0x00677d50`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14A04Unlink {
    /// Index of the matched node inside the modelled list order.
    pub index: usize,
    /// `+0x04` of the matched node: the previous link.
    pub prev: Option<u32>,
    /// `+0x00` of the matched node: the next link.
    pub next: Option<u32>,
    /// Interned-string count after the release.
    pub remaining: u32,
}

/// `0x00677d50`: release one interned string.
///
/// ```c
/// if (ptr < pool->base || pool->base + pool->count <= ptr) {
///   for (node = list->first; node != list->header; node = node->next)
///     if (ptr == node->text) {
///       free(node->text, 0x14);
///       next = node->next;
///       node->prev->next = next;
///       next->prev = node->prev;
///       pool->count--;
///       free(node, 0x14);
///       return;
///     }
/// }
/// ```
///
/// The early return is the important part: a pointer inside the pool's active
/// range is *not* an interned copy and is left alone.  `blocks` is the list in
/// link order, and `None` is the header.  The freed payload and node sizes are
/// [`W14A04_LIST_NODE_BYTES`]; the two link writes are reported instead of
/// performed so the caller owns the list.
pub fn w14a04_remove_interned(
    pool: (u32, u32),
    ptr: u32,
    blocks: &[W14A04PoolBlock],
    count: u32,
) -> Option<W14A04Unlink> {
    let (base, length) = pool;
    if ptr >= base && ptr < base.wrapping_add(length) {
        return None;
    }
    blocks
        .iter()
        .enumerate()
        .find(|(_, node)| node.text == ptr)
        .map(|(index, node)| W14A04Unlink {
            index,
            prev: node.prev,
            next: node.next,
            remaining: count.saturating_sub(1),
        })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests_wave14_agent04 {
    use super::*;

    #[test]
    fn always_true_is_the_constant_one() {
        assert!(w14a04_always_true());
    }

    #[test]
    fn bool_text_uses_the_shipped_literals() {
        assert_eq!(w14a04_bool_text(true), "true");
        assert_eq!(w14a04_bool_text(false), "false");
        assert_eq!(w14a04_bool_text(true).len(), 4);
        assert_eq!(w14a04_bool_text(false).len(), 5);
    }

    #[test]
    fn ci_less_folds_only_ascii_upper_case() {
        // Equal strings and prefixes.
        assert!(!w14a04_ci_less(b"", b""));
        assert!(w14a04_ci_less(b"", b"a"));
        assert!(!w14a04_ci_less(b"a", b""));
        assert!(!w14a04_ci_less(b"abc", b"abc"));
        assert!(w14a04_ci_less(b"abc", b"abcd"));
        assert!(!w14a04_ci_less(b"abcd", b"abc"));

        // Case folding on both sides.
        assert!(!w14a04_ci_less(b"ABC", b"abc"));
        assert!(!w14a04_ci_less(b"abc", b"ABC"));
        assert!(w14a04_ci_less(b"ABC", b"abd"));
        assert!(w14a04_ci_less(b"apple", b"Zebra"));
        assert!(!w14a04_ci_less(b"Zebra", b"apple"));

        // The shipped byte test folds exactly 'A'..='Z'.
        assert_eq!(w14a04_ci_fold(b'A'), b'a' as i8);
        assert_eq!(w14a04_ci_fold(b'Z'), b'z' as i8);
        assert_eq!(w14a04_ci_fold(b'@'), b'@' as i8);
        assert_eq!(w14a04_ci_fold(b'['), b'[' as i8);
        assert!(w14a04_ci_less(b"@", b"a"));
        assert!(!w14a04_ci_less(b"Z", b"["));
        assert!(w14a04_ci_less(b"[", b"Z"));

        // Ordering compares signed bytes, so high bytes sort before ASCII.
        assert_eq!(w14a04_ci_fold(0x80), -128);
        assert!(w14a04_ci_less(&[0x80], b"\x7f"));
        assert!(!w14a04_ci_less(b"\x7f", &[0x80]));
    }

    fn w14a04_node_with_key<'a>(key: &'a [u8], word: u32) -> W14A04Node<'a> {
        let mut node = w14a04_node_new([word, word + 1, word + 2, word + 3]);
        node.key = key;
        node
    }

    #[test]
    fn node_new_copies_the_four_words_and_clears_the_colour() {
        let node = w14a04_node_with_key(b"alpha", 7);
        assert_eq!(node.words, [7, 8, 9, 10]);
        assert!(!node.black);
        assert!(!node.nil);
        assert_eq!(node.left, None);
        assert_eq!(node.parent, None);
        assert_eq!(node.right, None);
        assert_eq!(node.key, b"alpha");

        // The clear store is one 16-bit write at +0x0c, i.e. colour and
        // _Isnil together.
        let raw = w14a04_node_new([0, 0, 0, 0]);
        assert_eq!((raw.black, raw.nil), (false, false));
    }

    #[test]
    fn small_node_new_keeps_the_links_and_three_words() {
        let node = w14a04_small_node_new(Some(9), Some(4), [1, 2, 3]);
        assert_eq!(node.next, Some(9));
        assert_eq!(node.prev, Some(4));
        assert_eq!(node.words, [1, 2, 3]);
        let head = w14a04_small_node_new(None, None, [0, 0, 0]);
        assert_eq!(head.next, None);
    }

    fn w14a04_build<'a>(keys: &'a [Vec<u8>]) -> (W14A04Tree, Vec<W14A04Node<'a>>) {
        let mut tree = W14A04Tree::default();
        let mut nodes: Vec<W14A04Node<'a>> = Vec::new();
        for key in keys {
            let index = nodes.len() as u32;
            nodes.push(w14a04_node_new([index, 0, 0, 0]));
            nodes[index as usize].key = key.as_slice();
            w14a04_insert_at(&mut tree, &mut nodes, false, key.as_slice(), index)
                .expect("unique insert");
        }
        (tree, nodes)
    }

    fn w14a04_black_height(nodes: &[W14A04Node<'_>], node: Option<u32>) -> usize {
        let Some(index) = node else {
            return 1;
        };
        let current = &nodes[index as usize];
        let left = w14a04_black_height(nodes, current.left);
        let right = w14a04_black_height(nodes, current.right);
        assert_eq!(left, right, "black height differs below node {index}");
        if !current.black {
            for child in [current.left, current.right].into_iter().flatten() {
                assert!(
                    nodes[child as usize].black,
                    "red node {index} has a red child {child}"
                );
            }
        }
        if let Some(child) = current.left {
            assert_eq!(nodes[child as usize].parent, Some(index));
            assert!(w14a04_ci_less(nodes[child as usize].key, current.key));
        }
        if let Some(child) = current.right {
            assert_eq!(nodes[child as usize].parent, Some(index));
            assert!(w14a04_ci_less(current.key, nodes[child as usize].key));
        }
        left + usize::from(current.black)
    }

    fn w14a04_check_tree(nodes: &[W14A04Node<'_>], tree: &W14A04Tree) {
        let Some(root) = tree.root else {
            assert_eq!(tree.size, 0);
            assert_eq!(tree.leftmost, None);
            assert_eq!(tree.rightmost, None);
            return;
        };
        assert!(nodes[root as usize].black, "the root must be black");
        assert_eq!(tree.leftmost, w14a04_leftmost(nodes, Some(root)));
        assert_eq!(tree.rightmost, w14a04_rightmost(nodes, Some(root)));
        w14a04_black_height(nodes, Some(root));
    }

    fn w14a04_in_order<'a>(nodes: &[W14A04Node<'a>], tree: &W14A04Tree) -> Vec<&'a [u8]> {
        let mut out = Vec::new();
        let mut cursor = tree.leftmost;
        while let Some(index) = cursor {
            out.push(nodes[index as usize].key);
            cursor = w14a04_next(nodes, index);
        }
        out
    }

    fn w14a04_shuffled_keys(count: usize) -> Vec<Vec<u8>> {
        let mut keys: Vec<Vec<u8>> = (0..count)
            .map(|index| format!("key{index:03}").into_bytes())
            .collect();
        let mut state = 0x1234_5678u32;
        for index in (1..keys.len()).rev() {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let swap = (state >> 8) as usize % (index + 1);
            keys.swap(index, swap);
        }
        keys
    }

    #[test]
    fn lower_bound_returns_the_first_key_not_less_than_the_probe() {
        let keys: Vec<Vec<u8>> = ["b", "d", "f", "h"]
            .iter()
            .map(|key| key.as_bytes().to_vec())
            .collect();
        let (tree, nodes) = w14a04_build(&keys);
        let ordered = w14a04_in_order(&nodes, &tree);
        assert_eq!(ordered, vec![&b"b"[..], &b"d"[..], &b"f"[..], &b"h"[..]]);

        let found = w14a04_lower_bound(tree.root, &nodes, b"a").expect("first slot");
        assert_eq!(nodes[found as usize].key, b"b");
        let found = w14a04_lower_bound(tree.root, &nodes, b"b").expect("exact slot");
        assert_eq!(nodes[found as usize].key, b"b");
        let found = w14a04_lower_bound(tree.root, &nodes, b"c").expect("next slot");
        assert_eq!(nodes[found as usize].key, b"d");
        let found = w14a04_lower_bound(tree.root, &nodes, b"h").expect("last slot");
        assert_eq!(nodes[found as usize].key, b"h");
        assert_eq!(w14a04_lower_bound(None, &nodes, b"a"), None);

        // The probe is folded: an upper-case probe finds the lower-case key.
        let found = w14a04_lower_bound(tree.root, &nodes, b"F").expect("folded slot");
        assert_eq!(nodes[found as usize].key, b"f");

        // A probe past every key leaves the result on the header, i.e. `None`.
        assert_eq!(w14a04_lower_bound(tree.root, &nodes, b"i"), None);
        assert_eq!(w14a04_lower_bound(tree.root, &nodes, b"zzz"), None);
    }

    #[test]
    fn next_walks_the_tree_in_order_and_stops_at_the_header() {
        let keys = w14a04_shuffled_keys(24);
        let (tree, nodes) = w14a04_build(&keys);
        let mut sorted: Vec<&[u8]> = keys.iter().map(|key| key.as_slice()).collect();
        sorted.sort_unstable();
        assert_eq!(w14a04_in_order(&nodes, &tree), sorted);
        assert_eq!(
            w14a04_next(&nodes, tree.rightmost.expect("rightmost")),
            None
        );
        // A leaf without a right child walks up through its parents.
        let leaf = tree.leftmost.expect("leftmost");
        let successor = w14a04_next(&nodes, leaf).expect("successor");
        assert_eq!(nodes[successor as usize].key, sorted[1]);
    }

    #[test]
    fn insert_links_the_first_node_into_the_header() {
        let mut nodes: Vec<W14A04Node<'static>> = vec![
            w14a04_node_with_key(b"m", 1),
            w14a04_node_with_key(b"c", 5),
            w14a04_node_with_key(b"x", 9),
        ];
        let mut tree = W14A04Tree::default();
        assert_eq!(w14a04_insert(&mut tree, &mut nodes, true, None, 0), Ok(0));
        assert_eq!(
            (tree.root, tree.leftmost, tree.rightmost, tree.size),
            (Some(0), Some(0), Some(0), 1)
        );
        assert_eq!(nodes[0].parent, None);
        assert!(nodes[0].black, "the fix-up blackens the root");

        assert_eq!(
            w14a04_insert(&mut tree, &mut nodes, true, Some(0), 1),
            Ok(1)
        );
        assert_eq!(nodes[0].left, Some(1));
        assert_eq!(
            (tree.leftmost, tree.rightmost, tree.size),
            (Some(1), Some(0), 2)
        );

        assert_eq!(
            w14a04_insert(&mut tree, &mut nodes, false, Some(0), 2),
            Ok(2)
        );
        assert_eq!(nodes[0].right, Some(2));
        assert_eq!(
            (tree.leftmost, tree.rightmost, tree.size),
            (Some(1), Some(2), 3)
        );
        w14a04_check_tree(&nodes, &tree);
    }

    #[test]
    fn insert_rejects_the_recovered_node_count_ceiling() {
        let mut nodes: Vec<W14A04Node<'static>> =
            vec![w14a04_node_with_key(b"a", 1), w14a04_node_with_key(b"b", 2)];
        let mut tree = W14A04Tree {
            size: W14A04_MAX_NODES - 1,
            ..W14A04Tree::default()
        };
        assert_eq!(w14a04_insert(&mut tree, &mut nodes, true, None, 0), Ok(0));
        assert_eq!(tree.size, W14A04_MAX_NODES);

        let mut full = W14A04Tree {
            size: W14A04_MAX_NODES,
            ..W14A04Tree::default()
        };
        assert_eq!(
            w14a04_insert(&mut full, &mut nodes, true, None, 1),
            Err(W14A04TreeError::TooLong)
        );
        assert_eq!(
            w14a04_insert(&mut full, &mut nodes, true, Some(7), 1),
            Err(W14A04TreeError::TooLong)
        );
        // An unknown node index is refused instead of dereferenced.
        let mut empty = W14A04Tree::default();
        assert_eq!(
            w14a04_insert(&mut empty, &mut nodes, true, None, 9),
            Err(W14A04TreeError::MissingNode(9))
        );
        assert_eq!(
            w14a04_insert(&mut empty, &mut nodes, true, Some(9), 0),
            Err(W14A04TreeError::MissingNode(9))
        );
    }

    #[test]
    fn insert_at_keeps_the_red_black_invariants_for_every_order() {
        let ascending: Vec<Vec<u8>> = (0..40)
            .map(|index| format!("a{index:02}").into_bytes())
            .collect();
        let descending: Vec<Vec<u8>> = ascending.iter().rev().cloned().collect();
        let shuffled = w14a04_shuffled_keys(64);

        for keys in [ascending, descending, shuffled] {
            let (tree, nodes) = w14a04_build(&keys);
            assert_eq!(tree.size, keys.len());
            assert_eq!(nodes.len(), keys.len());
            w14a04_check_tree(&nodes, &tree);
            let mut sorted: Vec<&[u8]> = keys.iter().map(|key| key.as_slice()).collect();
            sorted.sort_unstable();
            assert_eq!(w14a04_in_order(&nodes, &tree), sorted);
        }
    }

    #[test]
    fn tree_clone_copies_the_shape_and_the_headers() {
        let keys = w14a04_shuffled_keys(16);
        let (tree, nodes) = w14a04_build(&keys);
        let before = nodes.clone();
        let (clone, clone_nodes) = w14a04_tree_clone(&tree, &nodes);

        assert_eq!(nodes, before, "the source arena must not change");
        assert_eq!(clone.size, tree.size);
        assert_eq!(clone_nodes.len(), nodes.len());
        assert_eq!(
            w14a04_in_order(&clone_nodes, &clone),
            w14a04_in_order(&nodes, &tree)
        );
        assert_eq!(clone.root, Some(0), "pre-order appends the root first");
        w14a04_check_tree(&clone_nodes, &clone);
        // The copy is a pre-order rebuild, so the node indices are permuted;
        // compare the value words as a multiset instead of by index.
        let mut source_words: Vec<[u32; 4]> = nodes.iter().map(|node| node.words).collect();
        let mut clone_words: Vec<[u32; 4]> = clone_nodes.iter().map(|node| node.words).collect();
        source_words.sort_unstable();
        clone_words.sort_unstable();
        assert_eq!(clone_words, source_words);
        for (index, node) in clone_nodes.iter().enumerate() {
            assert!(!node.nil, "the copy clears _Isnil");
            if let Some(parent) = node.parent {
                assert!(parent < index as u32, "parents precede their children");
            }
        }
        // Every parent link has a matching child link.
        for (index, node) in clone_nodes.iter().enumerate() {
            for child in [node.left, node.right].into_iter().flatten() {
                assert_eq!(clone_nodes[child as usize].parent, Some(index as u32));
            }
        }

        // An empty tree clones to an empty tree with no leftmost/rightmost.
        let (empty, empty_nodes) = w14a04_tree_clone(&W14A04Tree::default(), &[]);
        assert_eq!(empty, W14A04Tree::default());
        assert!(empty_nodes.is_empty());
    }

    #[test]
    fn copy_nodes_returns_the_sentinel_for_a_missing_subtree() {
        let mut out = Vec::new();
        assert_eq!(w14a04_copy_nodes(&[], None, Some(0), &mut out), None);
        assert!(out.is_empty());
        // A dangling source index is refused rather than dereferenced.
        assert_eq!(w14a04_copy_nodes(&[], Some(3), None, &mut out), None);
    }

    #[test]
    fn node_new_with_tree_copy_constructs_the_nested_tree() {
        let keys: Vec<Vec<u8>> = ["x", "y"]
            .iter()
            .map(|key| key.as_bytes().to_vec())
            .collect();
        let (nested, nested_nodes) = w14a04_build(&keys);
        let built =
            w14a04_node_new_with_tree([0xabc, 0xdef, 0x123], b"outer", &nested, &nested_nodes);

        assert_eq!(built.node.words, [0xabc, 0xdef, 0x123, 0]);
        assert_eq!(built.node.key, b"outer");
        assert_eq!(built.node.left, None);
        assert!(!built.node.black);
        assert_eq!(built.nested.size, 2);
        assert_eq!(
            w14a04_in_order(&built.nested_nodes, &built.nested),
            vec![&b"x"[..], &b"y"[..]]
        );
        w14a04_check_tree(&built.nested_nodes, &built.nested);
    }

    #[test]
    fn tidy_clears_the_count_and_the_root_links_only() {
        let keys = w14a04_shuffled_keys(8);
        let (mut tree, nodes) = w14a04_build(&keys);
        let rightmost = tree.rightmost;
        assert!(rightmost.is_some());
        w14a04_tidy(&mut tree);
        assert_eq!(tree.root, None);
        assert_eq!(tree.leftmost, None);
        assert_eq!(tree.size, 0);
        // The shipped _Tidy never writes header->right.
        assert_eq!(tree.rightmost, rightmost);
        assert_eq!(nodes.len(), 8);
    }

    #[test]
    fn owned_block_release_uses_the_zero_x20_size_class() {
        assert_eq!(w14a04_release_owned_block(0x1234_5678), (0x1234_5678, 0x20));
        assert_eq!(W14A04_NODE_BYTES, 0x20);
        assert_eq!(W14A04_LIST_NODE_BYTES, 0x14);
    }

    #[test]
    fn shared_ptr_manager_dtor_stores_the_wave13_vtable_identity() {
        let mut manager = W14A04SharedPtrManager::default();
        assert!(!w14a04_shared_ptr_manager_dtor(&mut manager, 0));
        assert_eq!(manager.vtable, 0x00d3_287c);
        assert!(!w14a04_shared_ptr_manager_dtor(&mut manager, 0xfe));
        assert!(w14a04_shared_ptr_manager_dtor(&mut manager, 1));
        assert!(w14a04_shared_ptr_manager_dtor(&mut manager, 0xff));
    }

    #[test]
    fn teardown_plan_matches_the_recovered_array_and_flags() {
        let plan = w14a04_teardown_plan((0x1000, 3), 0, 3, Some((1, 1)));
        assert_eq!(plan.records, (0x1000, 0x1000 + 3 * 0x1c));
        assert_eq!(plan.records_len, 3);
        assert!(plan.array_released);
        assert!(plan.second_released);
        assert_eq!(
            plan.refs,
            Some(W14A04RefCountRelease {
                first_zero: true,
                second_after: Some(0),
                second_zero: true,
            })
        );

        let plan = w14a04_teardown_plan((0x2000, 0), -2, -1, None);
        assert_eq!(plan.records, (0x2000, 0x2000));
        assert!(!plan.array_released);
        assert!(!plan.second_released);
        assert_eq!(plan.refs, None);

        // A two-count object: the first release does not reach zero.
        let rel = w14a04_release_refcounted(2, 1);
        assert!(!rel.first_zero);
        assert_eq!(rel.second_after, None);
        assert!(!rel.second_zero);
        // The first release reaches zero but the second one does not.
        let rel = w14a04_release_refcounted(1, 5);
        assert!(rel.first_zero);
        assert_eq!(rel.second_after, Some(4));
        assert!(!rel.second_zero);
        // A zero count wraps, exactly as the `lock xadd` sequence would.
        let rel = w14a04_release_refcounted(0, 1);
        assert!(!rel.first_zero);
    }

    #[test]
    fn interned_release_ignores_in_pool_pointers_and_unlinks_otherwise() {
        let blocks = [
            W14A04PoolBlock {
                next: Some(2),
                prev: None,
                text: 0x9000,
            },
            W14A04PoolBlock {
                next: Some(3),
                prev: Some(1),
                text: 0x9100,
            },
            W14A04PoolBlock {
                next: None,
                prev: Some(2),
                text: 0x9200,
            },
        ];
        // Inside the pool's active range: no release.
        assert_eq!(
            w14a04_remove_interned((0x9000, 0x40), 0x9010, &blocks, 3),
            None
        );
        // Outside the range and present: report the unlink.
        assert_eq!(
            w14a04_remove_interned((0x9000, 0x40), 0x9100, &blocks, 3),
            Some(W14A04Unlink {
                index: 1,
                prev: Some(1),
                next: Some(3),
                remaining: 2,
            })
        );
        // Outside the range and absent: nothing to do.
        assert_eq!(
            w14a04_remove_interned((0x9000, 0x40), 0x9300, &blocks, 3),
            None
        );
        // The upper bound is exclusive.
        assert_eq!(
            w14a04_remove_interned((0x9000, 0x40), 0x9040, &blocks, 3),
            None
        );
        // A zero-length pool never owns a pointer.
        assert_eq!(
            w14a04_remove_interned((0x9000, 0), 0x9000, &blocks, 3),
            Some(W14A04Unlink {
                index: 0,
                prev: None,
                next: Some(2),
                remaining: 2,
            })
        );
        assert_eq!(
            w14a04_remove_interned((0x9000, 1), 0x9000, &blocks, 0),
            None
        );
    }
}
