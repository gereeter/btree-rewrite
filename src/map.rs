
use std::io::{stderr, Write};
use std::fmt::Debug;

use collections::borrow::Borrow;
use core::mem;

use node::{self, NodeRef, Handle, marker};
use search;

use node::InsertResult::*;
use node::ForceResult::*;
use search::SearchResult::*;


/// A map based on a B-Tree.
///
/// B-Trees represent a fundamental compromise between cache-efficiency and actually minimizing
/// the amount of work performed in a search. In theory, a binary search tree (BST) is the optimal
/// choice for a sorted map, as a perfectly balanced BST performs the theoretical minimum amount of
/// comparisons necessary to find an element (log<sub>2</sub>n). However, in practice the way this
/// is done is *very* inefficient for modern computer architectures. In particular, every element
/// is stored in its own individually heap-allocated node. This means that every single insertion
/// triggers a heap-allocation, and every single comparison should be a cache-miss. Since these
/// are both notably expensive things to do in practice, we are forced to at very least reconsider
/// the BST strategy.
///
/// A B-Tree instead makes each node contain B-1 to 2B-1 elements in a contiguous array. By doing
/// this, we reduce the number of allocations by a factor of B, and improve cache efficiency in
/// searches. However, this does mean that searches will have to do *more* comparisons on average.
/// The precise number of comparisons depends on the node search strategy used. For optimal cache
/// efficiency, one could search the nodes linearly. For optimal comparisons, one could search
/// the node using binary search. As a compromise, one could also perform a linear search
/// that initially only checks every i<sup>th</sup> element for some choice of i.
///
/// Currently, our implementation simply performs naive linear search. This provides excellent
/// performance on *small* nodes of elements which are cheap to compare. However in the future we
/// would like to further explore choosing the optimal search strategy based on the choice of B,
/// and possibly other factors. Using linear search, searching for a random element is expected
/// to take O(B log<sub>B</sub>n) comparisons, which is generally worse than a BST. In practice,
/// however, performance is excellent.
///
/// It is a logic error for a key to be modified in such a way that the key's ordering relative to
/// any other key, as determined by the `Ord` trait, changes while it is in the map. This is
/// normally only possible through `Cell`, `RefCell`, global state, I/O, or unsafe code.
pub struct BTreeMap<K, V> {
    root: node::Root<K, V>,
    length: usize
}

/// An iterator over a BTreeMap's entries.
pub struct Iter<'a, K: 'a, V: 'a> {
    handle: Handle<NodeRef<'a, K, V, marker::Immut, marker::Leaf>, marker::Edge>
}

impl<K: Debug, V: Debug> BTreeMap<K, V> {
    pub fn dump(&self) {
        fn dump_node<'a, K: Debug + 'a, V: Debug + 'a>(node: NodeRef<'a, K, V, marker::Immut, marker::LeafOrInternal>, max_height: usize) {
            let indent = (max_height - node.height()) * 2;
            for _ in 0..indent { write!(stderr(), " ").unwrap(); }
            writeln!(stderr(), "At node with height {}, idx {}", node.height(), node.parent_idx()).unwrap();
            for _ in 0..indent { write!(stderr(), " ").unwrap(); }
            writeln!(stderr(), "Keys: {:?}", node.keys()).unwrap();
            for _ in 0..indent { write!(stderr(), " ").unwrap(); }
            writeln!(stderr(), "Vals: {:?}", node.vals()).unwrap();

            if let Internal(node) = node.force() {
                writeln!(stderr(), "").unwrap();
                for i in 0..(node.len()+1) {
                    let handle = unsafe { Handle::new(node, i) };
                    dump_node(handle.descend(), max_height);
                }
            }
            for _ in 0..indent { write!(stderr(), " ").unwrap(); }
            writeln!(stderr(), "Done with node at height {}", node.height()).unwrap();
            writeln!(stderr(), "").unwrap();
        }
        dump_node(self.root.as_ref(), self.root.as_ref().height());
    }
}

impl<K: Ord, V> BTreeMap<K, V> {
    /// Makes a new empty BTreeMap with a reasonable choice for B.
    pub fn new() -> Self {
        BTreeMap {
            root: node::Root::new_leaf(),
            length: 0
        }
    }

    /// Deprecated. Use `new` instead.
    pub fn with_b(_: usize) -> BTreeMap<K, V> {
        BTreeMap::new()
    }

    /// Clears the map, removing all values.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    ///
    /// let mut a = BTreeMap::new();
    /// a.insert(1, "a");
    /// a.clear();
    /// assert!(a.is_empty());
    /// ```
    pub fn clear(&mut self) {
        *self = BTreeMap::new()
    }
    
    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.get(&1), Some(&"a"));
    /// assert_eq!(map.get(&2), None);
    /// ```
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V> where K: Borrow<Q>, Q: Ord {
        match search::search_tree(self.root.as_ref(), key) {
            Found(handle) => Some(handle.into_kv().1),
            GoDown(_) => None
        }
    }

    /// Returns true if the map contains a value for the specified key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.insert(1, "a");
    /// assert_eq!(map.contains_key(&1), true);
    /// assert_eq!(map.contains_key(&2), false);
    /// ```
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool where K: Borrow<Q>, Q: Ord {
        self.get(key).is_some()
    }

    /// Returns a mutable reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering
    /// on the borrowed form *must* match the ordering on the key type.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.insert(1, "a");
    /// if let Some(x) = map.get_mut(&1) {
    ///     *x = "b";
    /// }
    /// assert_eq!(map[&1], "b");
    /// ```
    // See `get` for implementation notes, this is basically a copy-paste with mut's added
    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V> where K: Borrow<Q>, Q: Ord {
        match search::search_tree(self.root.as_mut(), key) {
            Found(handle) => Some(handle.into_kv_mut().1),
            GoDown(_) => None
        }
    }

    /// Inserts a key-value pair into the map. If the key already had a value
    /// present in the map, that value is returned. Otherwise, `None` is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// assert_eq!(map.insert(37, "a"), None);
    /// assert_eq!(map.is_empty(), false);
    ///
    /// map.insert(37, "b");
    /// assert_eq!(map.insert(37, "c"), Some("b"));
    /// assert_eq!(map[&37], "c");
    /// ```
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let mut ins_k;
        let mut ins_v;
        let mut ins_edge;

        {
            let insert_point = match search::search_tree(self.root.as_mut(), &key) {
                Found(handle) => return Some(mem::replace(handle.into_kv_mut().1, value)),
                GoDown(insert_point) => insert_point
            };

            self.length += 1;

            let mut cur_parent = match insert_point.insert(key, value) {
                Fit(_) => return None,
                Split(left, k, v, right) => {
                    ins_k = k;
                    ins_v = v;
                    ins_edge = right;
                    left.ascend().ok()
                }
            };

            loop {
                match cur_parent {
                    Some(parent) => match parent.insert(ins_k, ins_v, ins_edge) {
                        Fit(_) => return None,
                        Split(left, k, v, right) => {
                            ins_k = k;
                            ins_v = v;
                            ins_edge = right;
                            cur_parent = left.ascend().ok();
                        }
                    },
                    None => break
                }
            }
        }

        self.root.enlarge().push(ins_k, ins_v, ins_edge);

        None
    }
}

impl<'a, K: 'a, V: 'a> IntoIterator for &'a BTreeMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

impl<'a, K: 'a, V: 'a> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        let mut cur_handle = match self.handle.right_kv() {
            Ok(kv) => {
                let ret = kv.into_kv();
                self.handle = kv.right_edge();
                return Some(ret);
            },
            Err(last_edge) => match last_edge.into_node().ascend() {
                Ok(handle) => handle,
                Err(_) => return None
            }
        };

        loop {
            match cur_handle.right_kv() {
                Ok(kv) => {
                    let ret = kv.into_kv();
                    self.handle = first_leaf_edge(kv.right_edge().descend());
                    return Some(ret);
                },
                Err(last_edge) => match last_edge.into_node().ascend() {
                    Ok(new_handle) => cur_handle = new_handle,
                    Err(_) => return None
                }
            }
        }
    }
}

fn first_leaf_edge<'a, K: 'a, V: 'a, Mutability>(mut node: NodeRef<'a, K, V, Mutability, marker::LeafOrInternal>) -> Handle<NodeRef<'a, K, V, Mutability, marker::Leaf>, marker::Edge> {
    loop {
        match node.force() {
            Leaf(leaf) => return leaf.first_edge(),
            Internal(internal) => {
                node = internal.first_edge().descend();
            }
        }
    }
}

impl<K, V> BTreeMap<K, V> {
    /// Gets an iterator over the entries of the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    ///
    /// let mut map = BTreeMap::new();
    /// map.insert(1, "a");
    /// map.insert(2, "b");
    /// map.insert(3, "c");
    ///
    /// for (key, value) in map.iter() {
    ///     println!("{}: {}", key, value);
    /// }
    ///
    /// let (first_key, first_value) = map.iter().next().unwrap();
    /// assert_eq!((*first_key, *first_value), (1, "a"));
    /// ```
    pub fn iter(&self) -> Iter<K, V> {
        Iter {
            handle: first_leaf_edge(self.root.as_ref())
        }
    }

    /// Returns the number of elements in the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    ///
    /// let mut a = BTreeMap::new();
    /// assert_eq!(a.len(), 0);
    /// a.insert(1, "a");
    /// assert_eq!(a.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.length
    }


    /// Returns true if the map contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    ///
    /// let mut a = BTreeMap::new();
    /// assert!(a.is_empty());
    /// a.insert(1, "a");
    /// assert!(!a.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}
