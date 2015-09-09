// This is an attempt at an implementation following the ideal
//
// ```
// struct BTreeMap<K, V> {
//     height: usize,
//     root: Option<Box<Node<K, V, height>>>
// }
//   
// struct Node<K, V, height: usize> {
//     keys: [K; 2 * T - 1],
//     vals: [V; 2 * T - 1],
//     edges: if height > 0 {
//         [Box<Node<K, V, height - 1>>; 2 * T]
//     } else { () },
//     parent: *mut Node<K, V, height + 1>,
//     parent_idx: u16,
//     len: u16,
// }
// ```
//
// Since Rust doesn't acutally have dependent types and polymorphic recursion, we make do with lots of unsafety.

use alloc::heap;
use core::marker::PhantomData;
use core::mem;
use core::nonzero::NonZero;
use core::ptr;
use core::slice;

const T: usize = 6;

struct LeafNode<K, V> {
    keys: [K; 2 * T - 1],
    vals: [V; 2 * T - 1],
    parent: *mut InternalNode<K, V>,
    parent_idx: u16,
    len: u16,
}

impl<K, V> LeafNode<K, V> {
    unsafe fn new() -> Self {
        LeafNode {
            keys: mem::uninitialized(),
            vals: mem::uninitialized(),
            parent: ptr::null_mut(),
            parent_idx: mem::uninitialized(),
            len: 0
        }
    }
}

// We use repr(C) so that a pointer to an internal node can be directly used as a pointer to a leaf node
#[repr(C)]
struct InternalNode<K, V> {
    data: LeafNode<K, V>,
    edges: [BoxedNode<K, V>; 2 * T],
}

impl<K, V> InternalNode<K, V> {
    unsafe fn new() -> Self {
        InternalNode {
            data: LeafNode::new(),
            edges: mem::uninitialized()
        }
    }
}

struct BoxedNode<K, V> {
    ptr: NonZero<*mut u8>, // we don't know if this points to a leaf node or an internal node
    _marker: PhantomData<*mut (K, V)>
}

impl<K, V> Copy for BoxedNode<K, V> { }
impl<K, V> Clone for BoxedNode<K, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K, V> BoxedNode<K, V> {
    fn from_leaf(node: Box<LeafNode<K, V>>) -> Self {
        unsafe {
            BoxedNode { ptr: NonZero::new(Box::into_raw(node) as *mut u8), _marker: PhantomData }
        }
    }

    fn from_internal(node: Box<InternalNode<K, V>>) -> Self {
        unsafe {
            BoxedNode { ptr: NonZero::new(Box::into_raw(node) as *mut u8), _marker: PhantomData }
        }
    }
}

pub struct Root<K, V> {
    node: BoxedNode<K, V>,
    height: usize
}

unsafe impl<K: Sync, V: Sync> Sync for Root<K, V> { }
unsafe impl<K: Send, V: Send> Send for Root<K, V> { }

impl<K, V> Root<K, V> {
    pub fn new_leaf() -> Self {
        Root {
            node: BoxedNode::from_leaf(Box::new(unsafe { LeafNode::new() })),
            height: 0
        }
    }

    pub fn as_ref(&self) -> NodeRef<marker::Borrowed, K, V, marker::Immut, marker::LeafOrInternal> {
        NodeRef {
            height: self.height,
            node: self.node,
            root: self as *const _ as *mut _,
            _marker: PhantomData,
        }
    }

    pub fn as_mut(&mut self) -> NodeRef<marker::Borrowed, K, V, marker::Mut, marker::LeafOrInternal> {
        NodeRef {
            height: self.height,
            node: self.node,
            root: self as *mut _,
            _marker: PhantomData,
        }
    }

    pub fn into_ref(self) -> NodeRef<marker::Owned, K, V, marker::Mut, marker::LeafOrInternal> {
        NodeRef {
            height: self.height,
            node: self.node,
            root: ptr::null_mut(), // TODO: Is there anything better to do here?
            _marker: PhantomData,
        }
    }

    pub fn enlarge(&mut self) -> NodeRef<marker::Borrowed, K, V, marker::Mut, marker::Internal> {
        let mut new_node = Box::new(unsafe { InternalNode::new() });
        new_node.edges[0] = self.node;

        self.node = BoxedNode::from_internal(new_node);
        self.height += 1;

        let mut ret = NodeRef {
            height: self.height,
            node: self.node,
            root: self as *mut _,
            _marker: PhantomData
        };

        unsafe {
            ret.reborrow_mut().first_edge().correct_parent_link();
        }

        ret
    }

    pub fn shrink(&mut self) {
        debug_assert!(self.height > 0);

        let top = *self.node.ptr;

        self.node = unsafe { self.as_mut().cast_unchecked::<marker::Internal>().first_edge().descend().node };
        self.height -= 1;
        self.as_mut().as_leaf_mut().parent = ptr::null_mut();

        unsafe {
            heap::deallocate(top, mem::size_of::<InternalNode<K, V>>(), mem::align_of::<InternalNode<K, V>>());
        }
    }
}

pub struct NodeRef<Lifetime, K, V, Mutability, Type> {
    height: usize,
    node: BoxedNode<K, V>,
    root: *mut Root<K, V>,
    _marker: PhantomData<(Lifetime, Mutability, Type)>
}

impl<'a, K: 'a, V: 'a, Type> Copy for NodeRef<marker::Borrowed<'a>, K, V, marker::Immut, Type> { }
impl<'a, K: 'a, V: 'a, Type> Clone for NodeRef<marker::Borrowed<'a>, K, V, marker::Immut, Type> {
    fn clone(&self) -> Self {
        *self
    }
}

unsafe impl<Lifetime, K: Sync, V: Sync, Mutability, Type> Sync for NodeRef<Lifetime, K, V, Mutability, Type> { }

unsafe impl<'a, K: Sync + 'a, V: Sync + 'a, Type> Send for NodeRef<marker::Borrowed<'a>, K, V, marker::Immut, Type> { }
unsafe impl<'a, K: Send + 'a, V: Send + 'a, Type> Send for NodeRef<marker::Borrowed<'a>, K, V, marker::Mut, Type> { }
unsafe impl<K: Send, V: Send, Mutability, Type> Send for NodeRef<marker::Owned, K, V, Mutability, Type> { }

impl<Lifetime, K, V, Mutability> NodeRef<Lifetime, K, V, Mutability, marker::Internal> {
    fn as_internal(&self) -> &InternalNode<K, V> {
        unsafe {
            &*(*self.node.ptr as *const InternalNode<K, V>)
        }
    }
}

impl<Lifetime, K, V> NodeRef<Lifetime, K, V, marker::Mut, marker::Internal> {
    fn as_internal_mut(&mut self) -> &mut InternalNode<K, V> {
        unsafe {
            &mut *(*self.node.ptr as *mut InternalNode<K, V>)
        }
    }
}


impl<Lifetime, K, V, Mutability, Type> NodeRef<Lifetime, K, V, Mutability, Type> {
    #[cfg(test)]
    pub fn height(&self) -> usize {
        self.height
    }

    #[cfg(test)]
    pub fn parent_idx(&self) -> usize {
        self.as_leaf().parent_idx as usize
    }

    pub fn len(&self) -> usize {
        self.as_leaf().len as usize
    }

    pub fn capacity(&self) -> usize {
        2 * T - 1
    }

    fn reborrow<'a>(&'a self) -> NodeRef<marker::Borrowed<'a>, K, V, marker::Immut, Type> {
        NodeRef {
            height: self.height,
            node: self.node,
            root: self.root,
            _marker: PhantomData
        }
    }

    fn as_leaf(&self) -> &LeafNode<K, V> {
        unsafe {
            &*(*self.node.ptr as *const LeafNode<K, V>)
        }
    }

    pub fn keys(&self) -> &[K] {
        self.reborrow().into_slices().0
    }

    pub fn vals(&self) -> &[V] {
        self.reborrow().into_slices().1
    }

    pub fn ascend(self) -> Result<Handle<NodeRef<Lifetime, K, V, Mutability, marker::Internal>, marker::Edge>, Self> {
        if self.as_leaf().parent.is_null() {
            Err(self)
        } else {
            Ok(Handle {
                node: NodeRef {
                    height: self.height + 1,
                    node: BoxedNode {
                        ptr: unsafe { NonZero::new(self.as_leaf().parent as *mut u8) },
                        _marker: PhantomData
                    },
                    root: self.root,
                    _marker: PhantomData
                },
                idx: self.as_leaf().parent_idx as usize,
                _marker: PhantomData
            })
        }
    }

    pub fn first_edge(self) -> Handle<Self, marker::Edge> {
        unsafe { Handle::new(self, 0) }
    }

    pub fn last_edge(self) -> Handle<Self, marker::Edge> {
        let len = self.len();
        unsafe { Handle::new(self, len) }
    }
}

impl<K, V> NodeRef<marker::Owned, K, V, marker::Mut, marker::Leaf> {
    pub unsafe fn deallocate_and_ascend(mut self) -> Option<Handle<NodeRef<marker::Owned, K, V, marker::Mut, marker::Internal>, marker::Edge>> {
        let ptr = self.as_leaf_mut() as *mut LeafNode<K, V> as *mut u8;
        let ret = self.ascend().ok();
        heap::deallocate(ptr, mem::size_of::<LeafNode<K, V>>(), mem::align_of::<LeafNode<K, V>>());
        ret
    }
}

impl<K, V> NodeRef<marker::Owned, K, V, marker::Mut, marker::Internal> {
    pub unsafe fn deallocate_and_ascend(mut self) -> Option<Handle<NodeRef<marker::Owned, K, V, marker::Mut, marker::Internal>, marker::Edge>> {
        let ptr = self.as_internal_mut() as *mut InternalNode<K, V> as *mut u8;
        let ret = self.ascend().ok();
        heap::deallocate(ptr, mem::size_of::<InternalNode<K, V>>(), mem::align_of::<InternalNode<K, V>>());
        ret
    }
}

impl<Lifetime, K, V, Type> NodeRef<Lifetime, K, V, marker::Mut, Type> {
    unsafe fn cast_unchecked<NewType>(&mut self) -> NodeRef<marker::Borrowed, K, V, marker::Mut, NewType> {
        NodeRef {
            height: self.height,
            node: self.node,
            root: self.root,
            _marker: PhantomData
        }
    }

    unsafe fn reborrow_mut(&mut self) -> NodeRef<marker::Borrowed, K, V, marker::Mut, Type> {
        NodeRef {
            height: self.height,
            node: self.node,
            root: self.root,
            _marker: PhantomData
        }
    }

    fn as_leaf_mut(&mut self) -> &mut LeafNode<K, V> {
        unsafe {
            &mut *(*self.node.ptr as *mut LeafNode<K, V>)
        }
    }

    pub fn keys_mut(&mut self) -> &mut [K] {
        unsafe { self.reborrow_mut().into_slices_mut().0 }
    }

    pub fn vals_mut(&mut self) -> &mut [V] {
        unsafe { self.reborrow_mut().into_slices_mut().1 }
    }
}

impl<'a, K: 'a, V: 'a, Mutability, Type> NodeRef<marker::Borrowed<'a>, K, V, Mutability, Type> {
    pub fn into_slices(self) -> (&'a [K], &'a [V]) {
        unsafe {
            (
                slice::from_raw_parts(
                    self.as_leaf().keys.as_ptr(),
                    self.len()
                ),
                slice::from_raw_parts(
                    self.as_leaf().vals.as_ptr(),
                    self.len()
                )
            )
        }
    }
}

impl<'a, K: 'a, V: 'a, Type> NodeRef<marker::Borrowed<'a>, K, V, marker::Mut, Type> {
    pub fn into_root_mut(self) -> &'a mut Root<K, V> {
        unsafe {
            &mut *self.root
        }
    }

    pub fn into_slices_mut(mut self) -> (&'a mut [K], &'a mut [V]) {
        unsafe {
            (
                slice::from_raw_parts_mut(
                    &mut self.as_leaf_mut().keys as *mut [K] as *mut K,
                    self.len()
                ),
                slice::from_raw_parts_mut(
                    &mut self.as_leaf_mut().vals as *mut [V] as *mut V,
                    self.len()
                )
            )
        }
    }
}

impl<Lifetime, K, V> NodeRef<Lifetime, K, V, marker::Mut, marker::Internal> {
    pub fn push(&mut self, key: K, val: V, edge: Root<K, V>) {
        // Necessary for correctness, but this is an internal module
        debug_assert!(edge.height == self.height - 1);
        debug_assert!(self.len() < self.capacity());

        let idx = self.len();

        unsafe {
            ptr::write(self.keys_mut().get_unchecked_mut(idx), key);
            ptr::write(self.vals_mut().get_unchecked_mut(idx), val);
            ptr::write(self.as_internal_mut().edges.get_unchecked_mut(idx + 1), edge.node);
            mem::forget(edge);

            Handle::new(self.reborrow_mut(), idx + 1).correct_parent_link();
        }

        self.as_leaf_mut().len += 1;
    }
}

impl<Lifetime, K, V, Mutability> NodeRef<Lifetime, K, V, Mutability, marker::LeafOrInternal> {
    pub fn force(self) -> ForceResult<NodeRef<Lifetime, K, V, Mutability, marker::Leaf>, NodeRef<Lifetime, K, V, Mutability, marker::Internal>> {
        if self.height == 0 {
            ForceResult::Leaf(NodeRef {
                height: self.height,
                node: self.node,
                root: self.root,
                _marker: PhantomData
            })
        } else {
            ForceResult::Internal(NodeRef {
                height: self.height,
                node: self.node,
                root: self.root,
                _marker: PhantomData
            })
        }
    }
}

pub struct Handle<Node, Type> {
    node: Node,
    idx: usize,
    _marker: PhantomData<Type>
}

impl<Node: Copy, Type> Copy for Handle<Node, Type> { }
impl<Node: Copy, Type> Clone for Handle<Node, Type> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Node, Type> Handle<Node, Type> {
    pub unsafe fn new(node: Node, idx: usize) -> Self {
        Handle {
            node: node,
            idx: idx,
            _marker: PhantomData
        }
    }

    pub fn into_node(self) -> Node {
        self.node
    }
}

impl<Node> Handle<Node, marker::KV> {
    pub fn left_edge(self) -> Handle<Node, marker::Edge> {
        unsafe { Handle::new(self.node, self.idx) }
    }

    pub fn right_edge(self) -> Handle<Node, marker::Edge> {
        unsafe { Handle::new(self.node, self.idx + 1) }
    }
}

impl<Lifetime, K, V, Mutability, NodeType, HandleType> Handle<NodeRef<Lifetime, K, V, Mutability, NodeType>, HandleType> {
    pub fn reborrow(&self) -> Handle<NodeRef<marker::Borrowed, K, V, marker::Immut, NodeType>, HandleType> {
        unsafe { Handle::new(self.node.reborrow(), self.idx) }
    }
}

impl<Lifetime, K, V, Mutability, NodeType> Handle<NodeRef<Lifetime, K, V, Mutability, NodeType>, marker::Edge> {
    pub fn left_kv(self) -> Result<Handle<NodeRef<Lifetime, K, V, Mutability, NodeType>, marker::KV>, Self> {
        if self.idx > 0 {
            unsafe {
                Ok(Handle::new(self.node, self.idx - 1))
            }
        } else {
            Err(self)
        }
    }

    pub fn right_kv(self) -> Result<Handle<NodeRef<Lifetime, K, V, Mutability, NodeType>, marker::KV>, Self> {
        if self.idx < self.node.len() {
            unsafe {
                Ok(Handle::new(self.node, self.idx))
            }
        } else {
            Err(self)
        }
    }
}

impl<Lifetime, K, V> Handle<NodeRef<Lifetime, K, V, marker::Mut, marker::Leaf>, marker::Edge> {
    unsafe fn insert_unchecked(&mut self, key: K, val: V) -> *mut V {
        slice_insert(self.node.keys_mut(), self.idx, key);
        slice_insert(self.node.vals_mut(), self.idx, val);

        self.node.as_leaf_mut().len += 1;

        self.node.vals_mut().get_unchecked_mut(self.idx)
    }

    pub fn insert(mut self, key: K, val: V) -> (InsertResult<Lifetime, K, V, marker::Leaf>, *mut V) {
        if self.node.len() < self.node.capacity() {
            unsafe {
                let ptr = self.insert_unchecked(key, val);
                (InsertResult::Fit(Handle::new(self.node, self.idx)), ptr)
            }
        } else {
            let middle = unsafe { Handle::new(self.node, T) };
            let (mut left, k, v, mut right) = middle.split();
            let ptr = if self.idx <= T {
                unsafe {
                    Handle::new(left.reborrow_mut(), self.idx).insert_unchecked(key, val)
                }
            } else {
                unsafe {
                    Handle::new(right.as_mut().cast_unchecked::<marker::Leaf>(), self.idx - T - 1).insert_unchecked(key, val)
                }
            };
            (InsertResult::Split(left, k, v, right), ptr)
        }
    }
}

impl<Lifetime, K, V> Handle<NodeRef<Lifetime, K, V, marker::Mut, marker::Internal>, marker::Edge> {
    fn correct_parent_link(mut self) {
        let idx = self.idx as u16;
        let ptr = self.node.as_internal_mut() as *mut _;
        let mut child = self.descend();
        child.as_leaf_mut().parent = ptr;
        child.as_leaf_mut().parent_idx = idx;
    }

    unsafe fn cast_unchecked<NewType>(&mut self) -> Handle<NodeRef<marker::Borrowed, K, V, marker::Mut, NewType>, marker::Edge> {
        Handle::new(self.node.cast_unchecked(), self.idx)
    }

    unsafe fn insert_unchecked(&mut self, key: K, val: V, edge: Root<K, V>) {
        self.cast_unchecked::<marker::Leaf>().insert_unchecked(key, val);

        slice_insert(slice::from_raw_parts_mut(self.node.as_internal_mut().edges.as_mut_ptr(), self.node.len()), self.idx + 1, edge.node);
        mem::forget(edge);

        for i in (self.idx+1)..(self.node.len()+1) {
            Handle::new(self.node.reborrow_mut(), i).correct_parent_link();
        }
    }

    pub fn insert(mut self, key: K, val: V, edge: Root<K, V>) -> InsertResult<Lifetime, K, V, marker::Internal> {
        // Necessary for correctness, but this is an internal module
        debug_assert!(edge.height == self.node.height - 1);

        if self.node.len() < self.node.capacity() {
            unsafe {
                self.insert_unchecked(key, val, edge);
                InsertResult::Fit(Handle::new(self.node, self.idx))
            }
        } else {
            let middle = unsafe { Handle::new(self.node, T) };
            let (mut left, k, v, mut right) = middle.split();
            if self.idx <= T {
                unsafe {
                    Handle::new(left.reborrow_mut(), self.idx).insert_unchecked(key, val, edge);
                }
            } else {
                unsafe {
                    Handle::new(right.as_mut().cast_unchecked::<marker::Internal>(), self.idx - T - 1).insert_unchecked(key, val, edge);
                }
            }
            InsertResult::Split(left, k, v, right)
        }
    }
}

impl<Lifetime, K, V, Mutability> Handle<NodeRef<Lifetime, K, V, Mutability, marker::Internal>, marker::Edge> {
    pub fn descend(self) -> NodeRef<Lifetime, K, V, Mutability, marker::LeafOrInternal> {
        NodeRef {
            height: self.node.height - 1,
            node: unsafe { *self.node.as_internal().edges.get_unchecked(self.idx) },
            root: self.node.root,
            _marker: PhantomData
        }
    }
}

impl<'a, K: 'a, V: 'a, Mutability, NodeType> Handle<NodeRef<marker::Borrowed<'a>, K, V, Mutability, NodeType>, marker::KV> {
    pub fn into_kv(self) -> (&'a K, &'a V) {
        let (keys, vals) = self.node.into_slices();
        unsafe {
            (keys.get_unchecked(self.idx), vals.get_unchecked(self.idx))
        }
    }
}

impl<'a, K: 'a, V: 'a, NodeType> Handle<NodeRef<marker::Borrowed<'a>, K, V, marker::Mut, NodeType>, marker::KV> {
    pub fn into_kv_mut(self) -> (&'a mut K, &'a mut V) {
        let (mut keys, mut vals) = self.node.into_slices_mut();
        unsafe {
            (keys.get_unchecked_mut(self.idx), vals.get_unchecked_mut(self.idx))
        }
    }
}

impl<Lifetime, K, V, NodeType> Handle<NodeRef<Lifetime, K, V, marker::Mut, NodeType>, marker::KV> {
    pub fn kv_mut(&mut self) -> (&mut K, &mut V) {
        unsafe {
            let (mut keys, mut vals) = self.node.reborrow_mut().into_slices_mut();
            (keys.get_unchecked_mut(self.idx), vals.get_unchecked_mut(self.idx))
        }
    }
}

impl<Lifetime, K, V> Handle<NodeRef<Lifetime, K, V, marker::Mut, marker::Leaf>, marker::KV> {
    pub fn split(mut self) -> (NodeRef<Lifetime, K, V, marker::Mut, marker::Leaf>, K, V, Root<K, V>) {
        unsafe {
            let mut new_node = Box::new(LeafNode::new());

            let k = ptr::read(self.node.keys().get_unchecked(self.idx));
            let v = ptr::read(self.node.vals().get_unchecked(self.idx));

            let new_len = self.node.len() - self.idx - 1;

            ptr::copy_nonoverlapping(
                self.node.keys().as_ptr().offset(self.idx as isize + 1),
                new_node.keys.as_mut_ptr(),
                new_len
            );
            ptr::copy_nonoverlapping(
                self.node.vals().as_ptr().offset(self.idx as isize + 1),
                new_node.vals.as_mut_ptr(),
                new_len
            );

            self.node.as_leaf_mut().len = self.idx as u16;
            new_node.len = new_len as u16;

            (
                self.node,
                k, v,
                Root {
                    node: BoxedNode::from_leaf(new_node),
                    height: 0
                }
            )
        }
    }

    pub fn remove(mut self) -> (Handle<NodeRef<Lifetime, K, V, marker::Mut, marker::Leaf>, marker::Edge>, K, V) {
        unsafe {
            let k = slice_remove(self.node.keys_mut(), self.idx);
            let v = slice_remove(self.node.vals_mut(), self.idx);
            self.node.as_leaf_mut().len -= 1;
            (self.left_edge(), k, v)
        }
    }
}

impl<Lifetime, K, V> Handle<NodeRef<Lifetime, K, V, marker::Mut, marker::Internal>, marker::KV> {
    pub fn split(mut self) -> (NodeRef<Lifetime, K, V, marker::Mut, marker::Internal>, K, V, Root<K, V>) {
        unsafe {
            let mut new_node = Box::new(InternalNode::new());

            let k = ptr::read(self.node.keys().get_unchecked(self.idx));
            let v = ptr::read(self.node.vals().get_unchecked(self.idx));

            let height = self.node.height;
            let new_len = self.node.len() - self.idx - 1;

            ptr::copy_nonoverlapping(
                self.node.keys().as_ptr().offset(self.idx as isize + 1),
                new_node.data.keys.as_mut_ptr(),
                new_len
            );
            ptr::copy_nonoverlapping(
                self.node.vals().as_ptr().offset(self.idx as isize + 1),
                new_node.data.vals.as_mut_ptr(),
                new_len
            );
            ptr::copy_nonoverlapping(
                self.node.as_internal().edges.as_ptr().offset(self.idx as isize + 1),
                new_node.edges.as_mut_ptr(),
                new_len + 1
            );

            self.node.as_leaf_mut().len = self.idx as u16;
            new_node.data.len = new_len as u16;

            let mut new_root = Root {
                node: BoxedNode::from_internal(new_node),
                height: height
            };
            
            for i in 0..(new_len+1) {
                Handle::new(new_root.as_mut().cast_unchecked(), i).correct_parent_link();
            }

            (
                self.node,
                k, v,
                new_root
            )
        }
    }
}

impl<Lifetime, K, V, Mutability, HandleType> Handle<NodeRef<Lifetime, K, V, Mutability, marker::LeafOrInternal>, HandleType> {
    pub fn force(self) -> ForceResult<Handle<NodeRef<Lifetime, K, V, Mutability, marker::Leaf>, HandleType>, Handle<NodeRef<Lifetime, K, V, Mutability, marker::Internal>, HandleType>> {
        match self.node.force() {
            ForceResult::Leaf(node) => ForceResult::Leaf(Handle {
                node: node,
                idx: self.idx,
                _marker: PhantomData
            }),
            ForceResult::Internal(node) => ForceResult::Internal(Handle {
                node: node,
                idx: self.idx,
                _marker: PhantomData
            })
        }
    }
}

pub enum ForceResult<Leaf, Internal> {
    Leaf(Leaf),
    Internal(Internal)
}

pub enum InsertResult<Lifetime, K, V, Type> {
    Fit(Handle<NodeRef<Lifetime, K, V, marker::Mut, Type>, marker::KV>),
    Split(NodeRef<Lifetime, K, V, marker::Mut, Type>, K, V, Root<K, V>)
}

pub mod marker {
    use core::marker::PhantomData;

    pub enum Leaf { }
    pub enum Internal { }
    pub enum LeafOrInternal { }

    pub enum Immut { }
    pub enum Mut { }

    pub enum KV { }
    pub enum Edge { }

    pub struct Borrowed<'a>(PhantomData<&'a mut ()>);
    pub enum Owned { }
}

unsafe fn slice_insert<T>(slice: &mut [T], idx: usize, val: T) {
    ptr::copy(
        slice.as_ptr().offset(idx as isize),
        slice.as_mut_ptr().offset(idx as isize + 1),
        slice.len() - idx
    );
    ptr::write(slice.get_unchecked_mut(idx), val);
}

unsafe fn slice_remove<T>(slice: &mut [T], idx: usize) -> T {
    let ret = ptr::read(slice.get_unchecked(idx));
    ptr::copy(
        slice.as_ptr().offset(idx as isize + 1),
        slice.as_mut_ptr().offset(idx as isize),
        slice.len() - idx - 1
    );
    ret
}
