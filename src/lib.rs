#![feature(core, collections, nonzero, collections_bound)]
#![feature(alloc, heap_api, core_intrinsics)]

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

extern crate collections;
extern crate core;
extern crate alloc;

mod node;
mod search;
pub mod map;
