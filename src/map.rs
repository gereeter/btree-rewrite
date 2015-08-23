
use collections::borrow::Borrow;
use core::mem;

use node;
use search;

use node::InsertResult::*;
use search::SearchResult::*;

pub struct BTreeMap<K, V> {
    root: node::Root<K, V>,
    length: usize
}

impl<K: Ord, V> BTreeMap<K, V> {
    pub fn new() -> Self {
        BTreeMap {
            root: node::Root::new_leaf(),
            length: 0
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    // Searching in a B-Tree is pretty straightforward.
    //
    // Start at the root. Try to find the key in the current node. If we find it, return it.
    // If it's not in there, follow the edge *before* the smallest key larger than
    // the search key. If no such key exists (they're *all* smaller), then just take the last
    // edge in the node. If we're in a leaf and we don't find our key, then it's not
    // in the tree.

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

