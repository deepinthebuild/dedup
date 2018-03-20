#![feature(nll)]

use std::sync::{RwLock};
use std::mem;
use std::collections::LinkedList;
use std::hash::{Hash, BuildHasher, Hasher};
use std::collections::hash_map::RandomState;
use std::default::Default;

const LOAD_FACTOR: usize = 80;
const DEFAULT_INITIAL_CAPACITY: usize = 16;
const DEFAULT_SEGMENT_COUNT: usize = 16;

struct HashEntry<K, V> {
    hash: u64,
    key: K,
    value: V,
}

struct Segment<K, V> {
    count: usize,
    table: Vec<LinkedList<HashEntry<K, V>>>,
}

impl<K: PartialEq + Eq, V> Segment<K, V> {
    fn new_with_capacity(capacity: usize) -> Self {
        let mut table = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            table.push(LinkedList::new());
        }
        let count = 0;
        Segment{
            count,
            table,
        }
    }
    fn insert(&mut self, key: K, value: V, hash: u64) -> Option<V> {
        let c = self.count + 1;
        if c * 100 > self.threshold() {
            self.expand();
        }

        let index = hash as usize & (self.table.len() - 1);
        let slot = &mut self.table[index];
        let entry_chain = slot.iter_mut();
        for entry in entry_chain {
            if entry.hash != hash {
                continue
            } else if entry.key != key {
                continue
            } else {
                self.count = c;
                return Some(mem::replace(&mut entry.value, value))
            }
        }
        slot.push_front(HashEntry{hash, key, value});
        None
    }

    fn get(&self, key: K, hash: u64) -> Option<&V> {
        let index = hash as usize & (self.table.len() - 1);
        let slot = &self.table[index];
        let entry_chain = slot.iter();
        for entry in entry_chain {
            if entry.hash != hash {
                continue
            } else if entry.key != key {
                continue
            } else {
                return Some(&entry.value)
            }
        }
        None
    }

    fn contains(&self, key: K, hash: u64) -> bool {
        let index = hash as usize & (self.table.len() - 1);
        let slot = &self.table[index];
        let entry_chain = slot.iter();
        for entry in entry_chain {
            if entry.hash != hash {
                continue
            } else if entry.key != key {
                continue
            } else {
                return true
            }
        }
        false
    }

    fn expand(&mut self) {
        let new_len = self.table.len() << 1;
        let mut new_table = Vec::with_capacity(new_len);
        for _ in 0..new_len {
            new_table.push(LinkedList::new());
        }
        let old_table = mem::replace(&mut self.table, new_table);

        for entry in old_table.into_iter().flat_map(|ll| ll) {
            let index = entry.hash as usize & (new_len - 1);
            self.table[index].push_front(entry);
        }
    }

    fn threshold(&self) -> usize {
        self.table.len() * LOAD_FACTOR
    }
}

pub struct ConcurrentHashMap<K, V, B: BuildHasher = RandomState> {
    segments: Vec<RwLock<Segment<K, V>>>,
    hash_builder: B,
}

impl<K: PartialEq + Eq + Hash, V> ConcurrentHashMap<K, V, RandomState> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<K: PartialEq + Eq + Hash, V, B: BuildHasher> ConcurrentHashMap<K, V, B> {
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let hash = self.hash(&key);
        let segment_index = self.get_segment(hash);
        self.segments[segment_index].write().unwrap().insert(key, value, hash)
    }

    pub fn contains(&self, key: K) -> bool {
        let hash = self.hash(&key);
        let segment_index = self.get_segment(hash);
        self.segments[segment_index].read().unwrap().contains(key, hash)
    }

    pub fn get(&mut self, key: K) -> Option<&V> {
        let hash = self.hash(&key);
        let segment_index = self.get_segment(hash);
        self.segments[segment_index].get_mut().unwrap().get(key, hash)
    }

    pub fn with_capacity_and_hasher_and_concurrencylevel(capacity: usize, hash_builder: B, concurrency_level: usize) -> Self {
        let concurrency_level = concurrency_level.next_power_of_two();
        let per_segment_capacity = (capacity / concurrency_level).next_power_of_two();
        let mut segments = Vec::with_capacity(concurrency_level);
        for _ in 0..concurrency_level {
            segments.push(RwLock::new(Segment::new_with_capacity(per_segment_capacity)))
        }
        ConcurrentHashMap{
            hash_builder,
            segments,
        }
    }

    fn hash(&self, key: &K) -> u64 {
        let mut hasher = self.hash_builder.build_hasher();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn get_segment(&self, hash: u64) -> usize {
        let shift_size = 64 - self.segments.len().trailing_zeros();
        (hash as usize >> shift_size) & (self.segments.len() - 1)
    }
}

impl<K: PartialEq + Eq + Hash, V, B: BuildHasher + Default> Default for ConcurrentHashMap<K, V, B> {
    fn default() -> Self {
        ConcurrentHashMap::with_capacity_and_hasher_and_concurrencylevel(DEFAULT_INITIAL_CAPACITY, Default::default(), DEFAULT_SEGMENT_COUNT)
    }
}

pub struct ConcurrentHashSet<K, B: BuildHasher = RandomState> {
    table: ConcurrentHashMap<K, (), B>,
}

impl<K: PartialEq + Eq + Hash> ConcurrentHashSet<K, RandomState> {
    pub fn new() -> Self {
        ConcurrentHashSet {
            table: ConcurrentHashMap::new()
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        ConcurrentHashSet{
            table: ConcurrentHashMap::with_capacity_and_hasher_and_concurrencylevel(capacity, Default::default(), DEFAULT_SEGMENT_COUNT)
        }
    }
}

impl<K: PartialEq + Eq + Hash, B: BuildHasher> ConcurrentHashSet<K, B> {
    pub fn insert(&self, key: K) -> bool {
        self.table.insert(key, ()).is_none()
    }

    pub fn contains(&self, key: K) -> bool {
        self.table.contains(key)
    }
}

impl<K: PartialEq + Eq + Hash, B: BuildHasher + Default> Default for ConcurrentHashSet<K, B> {
    fn default() -> Self {
        ConcurrentHashSet{
            table: ConcurrentHashMap::default()
        }
    }
}