extern crate btree_rewrite;
extern crate rand;

use btree_rewrite::BTreeMap;


#[test]
pub fn test_rand_large() {
    use rand::{thread_rng, Rng};

    let n: usize = 50;
    let mut map = BTreeMap::new();
    // setup
    let mut rng = thread_rng();
    let mut log = Vec::new();

    for _ in 0..n {
        let i = rng.gen::<usize>() % (2*n);
        if !map.contains_key(&i) {
            log.push(i);
        }
        map.insert(i, i);
//        map.dump();
    }


    for i in log {
        assert_eq!(map.get(&i).cloned(), Some(i));
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

#[test]
fn test_iter() {
    use rand::{thread_rng, Rng};
    let size = 10000;

    let mut rng = thread_rng();

    // Forwards
    // let map: BTreeMap<_, _> = (0..size).map(|i| (i, i)).collect();
    let mut map = BTreeMap::new();
    let mut log = Vec::new();
    for i in 0..size {
        let key = rng.gen();
        let val = rng.gen();
        if !map.contains_key(&key) {
            log.push((key, val));
        }
        map.insert(key, val);
    }
    log.sort();

    fn test<T>(size: usize, mut iter: T, log: Vec<(usize, usize)>) where T: Iterator<Item=(usize, usize)> {
        for (i, kv) in log.into_iter().enumerate() {
            println!("{}: kv={:?}", i, kv);
//            assert_eq!(iter.size_hint(), (size - i, Some(size - i)));
            assert_eq!(iter.next().unwrap(), kv);
        }
//        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert_eq!(iter.next(), None);
    }
    test(size, map.iter().map(|(&k, &v)| (k, v)), log);
}
