#![feature(test)]

extern crate btree_rewrite;
extern crate rand;
extern crate test;

macro_rules! map_insert_rand_bench {
    ($name: ident, $n: expr, $map: ident) => (
        #[bench]
        pub fn $name(b: &mut ::test::Bencher) {
            use rand::{thread_rng, Rng};
            use test::black_box;

            let n: usize = $n;
            let mut map = $map::new();
            // setup
            let mut rng = thread_rng();

            for _ in 0..n {
                let i = rng.gen::<usize>() % n;
                map.insert(i, i);
            }

            // measure
            b.iter(|| {
                let k = rng.gen::<usize>() % n;
                map.insert(k, k);
                //map.remove(&k);
            });
            black_box(map);
        }
    )
}

macro_rules! map_insert_seq_bench {
    ($name: ident, $n: expr, $map: ident) => (
        #[bench]
        pub fn $name(b: &mut ::test::Bencher) {
            use test::black_box;

            let mut map = $map::new();
            let n: usize = $n;
            // setup
            for i in 0..n {
                map.insert(i * 2, i * 2);
            }

            // measure
            let mut i = 1;
            b.iter(|| {
                map.insert(i, i);
                //map.remove(&i);
                i = (i + 2) % n;
            });
            black_box(map);
        }
    )
}

macro_rules! map_find_rand_bench {
    ($name: ident, $n: expr, $map: ident) => (
        #[bench]
        pub fn $name(b: &mut ::test::Bencher) {
            use rand::{thread_rng, Rng};
            use test::black_box;

            let mut map = $map::new();
            let n: usize = $n;

            // setup
            let mut rng = thread_rng();
            let mut keys: Vec<_> = (0..n).map(|_| rng.gen::<usize>() % n).collect();

            for &k in &keys {
                map.insert(k, k);
            }

            rng.shuffle(&mut keys);

            // measure
            let mut i = 0;
            b.iter(|| {
                let t = map.get(&keys[i]);
                i = (i + 1) % n;
                black_box(t);
            })
        }
    )
}

macro_rules! map_find_seq_bench {
    ($name: ident, $n: expr, $map: ident) => (
        #[bench]
        pub fn $name(b: &mut ::test::Bencher) {
            use test::black_box;

            let mut map = $map::new();
            let n: usize = $n;

            // setup
            for i in 0..n {
                map.insert(i, i);
            }

            // measure
            let mut i = 0;
            b.iter(|| {
                let x = map.get(&i);
                i = (i + 1) % n;
                black_box(x);
            })
        }
    )
}

macro_rules! map_iter_bench {
    ($name: ident, $n: expr, $map: ident) => (
        #[bench]
        fn $name(b: &mut ::test::Bencher) {
            use rand::{thread_rng, Rng};
            use test::black_box;

            let mut map = $map::<i32, i32>::new();
            let mut rng = thread_rng();

            for _ in 0..$n {
                map.insert(rng.gen(), rng.gen());
            }

            b.iter(|| {
                for entry in map.iter() {
                    black_box(entry);
                }
            });
        }
    )
}

use btree_rewrite::map::BTreeMap as ParentMap;
use std::collections::BTreeMap as StdMap;

map_insert_rand_bench!{insert_rand_100000_parent, 100_000, ParentMap}
map_insert_rand_bench!{insert_rand_100000_std   , 100_000, StdMap}
map_insert_rand_bench!{insert_rand_10000_parent ,  10_000, ParentMap}
map_insert_rand_bench!{insert_rand_10000_std    ,  10_000, StdMap}
map_insert_rand_bench!{insert_rand_100_parent   ,     100, ParentMap}
map_insert_rand_bench!{insert_rand_100_std      ,     100, StdMap}

map_insert_seq_bench!{insert_seq_100000_parent  , 100_000, ParentMap}
map_insert_seq_bench!{insert_seq_100000_std     , 100_000, StdMap}
map_insert_seq_bench!{insert_seq_10000_parent   ,  10_000, ParentMap}
map_insert_seq_bench!{insert_seq_10000_std      ,  10_000, StdMap}
map_insert_seq_bench!{insert_seq_100_parent     ,     100, ParentMap}
map_insert_seq_bench!{insert_seq_100_std        ,     100, StdMap}

map_find_rand_bench!{find_rand_100000_parent, 100_000, ParentMap}
map_find_rand_bench!{find_rand_100000_std   , 100_000, StdMap}
map_find_rand_bench!{find_rand_10000_parent ,  10_000, ParentMap}
map_find_rand_bench!{find_rand_10000_std    ,  10_000, StdMap}
map_find_rand_bench!{find_rand_100_parent   ,     100, ParentMap}
map_find_rand_bench!{find_rand_100_std      ,     100, StdMap}

map_find_seq_bench!{find_seq_100000_parent  , 100_000, ParentMap}
map_find_seq_bench!{find_seq_100000_std     , 100_000, StdMap}
map_find_seq_bench!{find_seq_10000_parent   ,  10_000, ParentMap}
map_find_seq_bench!{find_seq_10000_std      ,  10_000, StdMap}
map_find_seq_bench!{find_seq_100_parent     ,     100, ParentMap}
map_find_seq_bench!{find_seq_100_std        ,     100, StdMap}

map_iter_bench!{iter_100000_parent, 100_000, ParentMap}
map_iter_bench!{iter_100000_std   , 100_000, StdMap}
map_iter_bench!{iter_1000_parent  ,    1000, ParentMap}
map_iter_bench!{iter_1000_std     ,    1000, StdMap}
map_iter_bench!{iter_20_parent    ,      20, ParentMap}
map_iter_bench!{iter_20_std       ,      20, StdMap}
