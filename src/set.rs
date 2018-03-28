use poirot::ConcurrentHashSet;

use fxhash::FxHasher;

use std::collections::HashSet;
use std::hash::BuildHasherDefault;

pub type Set<T> = HashSet<T, BuildHasherDefault<FxHasher>>;
pub type ConcurrentSet<T> = ConcurrentHashSet<T>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use rayon::prelude::*;
    use byteorder::{LittleEndian, WriteBytesExt};

    lazy_static! {
        static ref DATA1: Vec<u8> = {
            (0..250u8).into_iter().collect()
        };
    }
    lazy_static! {
        static ref DATA2: Vec<u8> = {
            (0..250u8).into_iter().rev().collect()
        };
    }

    lazy_static! {
        static ref DATA3: Vec<u8> = {
            (0..100).into_iter().cycle().take(10000).collect()
        };
    }

    #[test]
    fn concurrent_set_insert_test() {
        let data = vec![5u8, 100u8, 200u8];
        let set = ConcurrentSet::default();
        set.insert(&data[..]);

        assert!(set.contains(&data[..]));
    }

    #[test]
    fn concurrent_set_realloc_test() {
        let data: Vec<u8> = (0..200u8).into_iter().collect();
        let set = ConcurrentSet::default();
        for s in data.windows(2) {
            set.insert(s);
        }

        for s in data.windows(2) {
            assert!(set.contains(s));
        }
    }

    #[test]
    fn concurrent_inserts_test() {
        let set = Arc::new(ConcurrentSet::default());

        let handle1 = Arc::clone(&set);
        let child1 = thread::spawn( move || {
            for s in DATA1.windows(2) {
                handle1.insert(s);
            }
        });

        let handle2 = Arc::clone(&set);
        let child2 = thread::spawn( move || {
            for s in DATA2.windows(2) {
                handle2.insert(s);
            }
        });

        child1.join().unwrap();
        child2.join().unwrap();

        for s in DATA1.windows(2) {
            assert!(set.contains(s));
        }

        for s in DATA2.windows(2) {
            assert!(set.contains(s));
        }


    }

    #[test]
    fn many_concurrent_inserts_test() {
        let set = Arc::new(ConcurrentSet::<&'static [u8]>::default());
        let mut handles = Vec::with_capacity(16);

        for t in 0..16 {
            let setarc = Arc::clone(&set);
            handles.push(
                thread::spawn(move || {
                    for s in DATA1.windows((t / 2) + 1) {
                        setarc.insert(s);
                    }
                })
            );
        }

        for handle in handles {
            handle.join().unwrap();
        }

        for t in 0..16 {
            for s in DATA1.windows((t / 2) + 1) {
                assert!(set.contains(s));
            }
        }

    }

    #[test]
    fn rayon_filter_test() {
        let set = ConcurrentSet::<&'static [u8]>::default();
        let results: Vec<&[u8]> = DATA3.par_windows(2).filter(|s| set.insert(&s)).collect();
        assert_eq!(results.len(), 100);
    }

    #[test]
    #[ignore]
    fn rayon_big_filter_test() {
        lazy_static! {
            static ref DATA4: Vec<u8> = {
                let mut data = Vec::new();
                for t in 0..50_000_000u32 {
                    data.write_u32::<LittleEndian>(t % 1_000_000).unwrap();
                }
                data
            };
        }
        let set = ConcurrentSet::<&'static [u8]>::default();
        let results: Vec<&[u8]> = DATA4.par_chunks(4).filter(|s| set.insert(s)).collect();
        assert_eq!(results.len(), 1_000_000);
    }

    #[test]
    fn detect_duplicates_test() {
        let set = ConcurrentSet::<&'static [u8]>::default();

        for s in DATA1.windows(3) {
            assert!(set.insert(s));
        }

        for s in DATA1.windows(3) {
            assert!(!set.insert(s));
        }
    }
}
