#![feature(test)]

extern crate btree_rewrite;
extern crate btree;
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

use btree::BTreeMap as ApaselMap;
use btree_rewrite::map::BTreeMap as ParentMap;
use std::collections::BTreeMap as StdMap;

map_insert_rand_bench!{rand_1000000_apasel, 1_000_000, ApaselMap}
map_insert_rand_bench!{rand_1000000_parent, 1_000_000, ParentMap}
map_insert_rand_bench!{rand_1000000_std   , 1_000_000, StdMap}
map_insert_rand_bench!{rand_100000_apasel, 100_000, ApaselMap}
map_insert_rand_bench!{rand_100000_parent, 100_000, ParentMap}
map_insert_rand_bench!{rand_100000_std   , 100_000, StdMap}
map_insert_rand_bench!{rand_10000_apasel ,  10_000, ApaselMap}
map_insert_rand_bench!{rand_10000_parent ,  10_000, ParentMap}
map_insert_rand_bench!{rand_10000_std    ,  10_000, StdMap}
map_insert_rand_bench!{rand_100_apasel   ,     100, ApaselMap}
map_insert_rand_bench!{rand_100_parent   ,     100, ParentMap}
map_insert_rand_bench!{rand_100_std      ,     100, StdMap}

map_insert_seq_bench!{seq_1000000_apasel  , 1_000_000, ApaselMap}
map_insert_seq_bench!{seq_1000000_parent  , 1_000_000, ParentMap}
map_insert_seq_bench!{seq_1000000_std     , 1_000_000, StdMap}
map_insert_seq_bench!{seq_100000_apasel  , 100_000, ApaselMap}
map_insert_seq_bench!{seq_100000_parent  , 100_000, ParentMap}
map_insert_seq_bench!{seq_100000_std     , 100_000, StdMap}
map_insert_seq_bench!{seq_10000_apasel   ,  10_000, ApaselMap}
map_insert_seq_bench!{seq_10000_parent   ,  10_000, ParentMap}
map_insert_seq_bench!{seq_10000_std      ,  10_000, StdMap}
map_insert_seq_bench!{seq_100_apasel     ,     100, ApaselMap}
map_insert_seq_bench!{seq_100_parent     ,     100, ParentMap}
map_insert_seq_bench!{seq_100_std        ,     100, StdMap}

map_iter_bench!{iter_100000_apasel, 100_000, ApaselMap}
map_iter_bench!{iter_100000_parent, 100_000, ParentMap}
map_iter_bench!{iter_100000_std   , 100_000, StdMap}
map_iter_bench!{iter_1000_apasel  ,    1000, ApaselMap}
map_iter_bench!{iter_1000_parent  ,    1000, ParentMap}
map_iter_bench!{iter_1000_std     ,    1000, StdMap}
map_iter_bench!{iter_20_apasel    ,      20, ApaselMap}
map_iter_bench!{iter_20_parent    ,      20, ParentMap}
map_iter_bench!{iter_20_std       ,      20, StdMap}
