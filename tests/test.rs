extern crate btree_rewrite;
extern crate rand;

use btree_rewrite::BTreeMap;


#[test]
pub fn test_rand_insert_large() {
    use rand::{thread_rng, Rng};

    let n: usize = 100000;
    let mut map = BTreeMap::new();
    // setup
    let mut rng = thread_rng();

    for _ in 0..n {
        let i = rng.gen::<usize>() % n;
        map.insert(i, i);
    }
}

#[test]
fn test_basic_large() {
    let mut map = BTreeMap::new();
    let size = 10000;
    assert_eq!(map.len(), 0);

    for i in 0..size {
        assert_eq!(map.insert(i, 10*i), None);
        assert_eq!(map.len(), i + 1);
    }

    for i in 0..size {
        assert_eq!(map.get(&i).unwrap(), &(i*10));
    }

    for i in size..size*2 {
        assert_eq!(map.get(&i), None);
    }

    for i in 0..size {
        assert_eq!(map.insert(i, 100*i), Some(10*i));
        assert_eq!(map.len(), size);
    }

    for i in 0..size {
        assert_eq!(map.get(&i).unwrap(), &(i*100));
    }
}

#[test]
fn test_basic_small() {
    let mut map = BTreeMap::new();
    assert_eq!(map.get(&1), None);
    assert_eq!(map.insert(1, 1), None);
    assert_eq!(map.get(&1), Some(&1));
    assert_eq!(map.insert(1, 2), Some(1));
    assert_eq!(map.get(&1), Some(&2));
    assert_eq!(map.insert(2, 4), None);
    assert_eq!(map.get(&2), Some(&4));
}
