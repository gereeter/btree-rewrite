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

use core::marker::PhantomData;
use core::nonzero::NonZero;
use core::slice;

const T: usize = 6;

struct LeafNode<K, V> {
    keys: [K; 2 * T - 1],
    vals: [V; 2 * T - 1],
    parent: *mut InternalNode<K, V>,
    parent_idx: u16,
    len: u16,
}

// We use repr(C) so that a pointer to an internal node can be directly used as a pointer to a leaf node
#[repr(C)]
struct InternalNode<K, V> {
    data: LeafNode<K, V>,
    edges: [BoxedNode<K, V>; 2 * T],
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

impl<K, V> Root<K, V> {
    pub fn as_ref(&self) -> NodeRef<K, V, marker::Immut, marker::LeafOrInternal> {
        NodeRef {
            height: self.height,
            node: self.node,
            _marker: PhantomData,
        }
    }

    pub fn as_mut(&mut self) -> NodeRef<K, V, marker::Mut, marker::LeafOrInternal> {
        NodeRef {
            height: self.height,
            node: self.node,
            _marker: PhantomData,
        }
    }
}

pub struct NodeRef<'a, K: 'a, V: 'a, Mutability, Type> {
    height: usize,
    node: BoxedNode<K, V>,
    _marker: PhantomData<(&'a mut (K, V), Mutability, Type)>
}

impl<'a, K: 'a, V: 'a, Type> Copy for NodeRef<'a, K, V, marker::Immut, Type> { }
impl<'a, K: 'a, V: 'a, Type> Clone for NodeRef<'a, K, V, marker::Immut, Type> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, K: 'a, V: 'a, Mutability> NodeRef<'a, K, V, Mutability, marker::Internal> {
    fn as_internal(&self) -> &InternalNode<K, V> {
        unsafe {
            &*(*self.node.ptr as *const InternalNode<K, V>)
        }
    }
}


impl<'a, K: 'a, V: 'a, Mutability, Type> NodeRef<'a, K, V, Mutability, Type> {
    fn reborrow(&self) -> NodeRef<K, V, marker::Immut, Type> {
        NodeRef {
            height: self.height,
            node: self.node,
            _marker: PhantomData
        }
    }

    fn as_leaf(&self) -> &LeafNode<K, V> {
        unsafe {
            &*(*self.node.ptr as *const LeafNode<K, V>)
        }
    }

    pub fn into_slices(self) -> (&'a [K], &'a [V]) {
        unsafe {
            (
                slice::from_raw_parts(
                    &self.as_leaf().keys as *const [K] as *const K,
                    self.as_leaf().len as usize
                ),
                slice::from_raw_parts(
                    &self.as_leaf().vals as *const [V] as *const V,
                    self.as_leaf().len as usize
                )
            )
        }
    }

    pub fn keys(&self) -> &[K] {
        self.reborrow().into_slices().0
    }

    pub fn ascend(self) -> Result<Handle<NodeRef<'a, K, V, Mutability, marker::Internal>, marker::Edge>, Self> {
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
                    _marker: PhantomData
                },
                idx: self.as_leaf().parent_idx,
                _marker: PhantomData
            })
        }
    }
}

impl<'a, K: 'a, V: 'a, Type> NodeRef<'a, K, V, marker::Mut, Type> {
    fn as_leaf_mut(&mut self) -> &mut LeafNode<K, V> {
        unsafe {
            &mut *(*self.node.ptr as *mut LeafNode<K, V>)
        }
    }

    pub fn into_slices_mut(mut self) -> (&'a mut [K], &'a mut [V]) {
        unsafe {
            (
                slice::from_raw_parts_mut(
                    &mut self.as_leaf_mut().keys as *mut [K] as *mut K,
                    self.as_leaf().len as usize
                ),
                slice::from_raw_parts_mut(
                    &mut self.as_leaf_mut().vals as *mut [V] as *mut V,
                    self.as_leaf().len as usize
                )
            )
        }
    }
}

impl<'a, K: 'a, V: 'a, Mutability> NodeRef<'a, K, V, Mutability, marker::LeafOrInternal> {
    pub fn force(self) -> ForceResult<NodeRef<'a, K, V, Mutability, marker::Leaf>, NodeRef<'a, K, V, Mutability, marker::Internal>> {
        if self.height == 0 {
            ForceResult::Leaf(NodeRef {
                height: self.height,
                node: self.node,
                _marker: PhantomData
            })
        } else {
            ForceResult::Internal(NodeRef {
                height: self.height,
                node: self.node,
                _marker: PhantomData
            })
        }
    }
}

pub struct Handle<Node, Type> {
    node: Node,
    idx: u16,
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
            idx: idx as u16,
            _marker: PhantomData
        }
    }

    pub fn into_node(self) -> Node {
        self.node
    }
}

impl<'a, K: 'a, V: 'a, Mutability> Handle<NodeRef<'a, K, V, Mutability, marker::Internal>, marker::Edge> {
    pub fn descend(self) -> NodeRef<'a, K, V, Mutability, marker::LeafOrInternal> {
        NodeRef {
            height: self.node.height - 1,
            node: unsafe { *self.node.as_internal().edges.get_unchecked(self.idx as usize) },
            _marker: PhantomData
        }
    }
}

impl<'a, K: 'a, V: 'a, Mutability, NodeType> Handle<NodeRef<'a, K, V, Mutability, NodeType>, marker::KV> {
    pub fn into_kv(self) -> (&'a K, &'a V) {
        let (keys, vals) = self.node.into_slices();
        unsafe {
            (keys.get_unchecked(self.idx as usize), vals.get_unchecked(self.idx as usize))
        }
    }
}

impl<'a, K: 'a, V: 'a, NodeType> Handle<NodeRef<'a, K, V, marker::Mut, NodeType>, marker::KV> {
    pub fn into_kv_mut(self) -> (&'a mut K, &'a mut V) {
        let (mut keys, mut vals) = self.node.into_slices_mut();
        unsafe {
            (keys.get_unchecked_mut(self.idx as usize), vals.get_unchecked_mut(self.idx as usize))
        }
    }
}

impl<'a, K: 'a, V: 'a, Mutability, HandleType> Handle<NodeRef<'a, K, V, Mutability, marker::LeafOrInternal>, HandleType> {
    pub fn force(self) -> ForceResult<Handle<NodeRef<'a, K, V, Mutability, marker::Leaf>, HandleType>, Handle<NodeRef<'a, K, V, Mutability, marker::Internal>, HandleType>> {
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

pub mod marker {
    pub enum Leaf { }
    pub enum Internal { }
    pub enum LeafOrInternal { }

    pub enum Immut { }
    pub enum Mut { }

    pub enum KV { }
    pub enum Edge { }
}
