use std::sync::{RwLock};
use std::hash::{Hash, BuildHasher, Hasher};
use std::collections::hash_map::{HashMap, RandomState};
use std::default::Default;
use std::borrow::Borrow;

const DEFAULT_INITIAL_CAPACITY: usize = 64;
const DEFAULT_SEGMENT_COUNT: usize = 16;

pub struct ConcurrentHashMap<K, V, B: BuildHasher = RandomState> {
    segments: Vec<RwLock<HashMap<K, V, B>>>,
    hash_builder: B,
}

impl<K: Eq + Hash, V> ConcurrentHashMap<K, V, RandomState> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<K: Eq + Hash, V, B: BuildHasher + Default> ConcurrentHashMap<K, V, B> {
    #[inline]
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let hash = self.hash(&key);
        let segment_index = self.get_segment(hash);
        self.segments[segment_index].write().unwrap().insert(key, value)
    }

    pub fn contains<Q: ?Sized>(&self, key: &Q) -> bool where K: Borrow<Q>, Q: Eq + Hash {
        let hash = self.hash(key);
        let segment_index = self.get_segment(hash);
        self.segments[segment_index].read().unwrap().contains_key(key)
    }

    pub fn get<Q: ?Sized>(&mut self, key: &Q) -> Option<&V> where K: Borrow<Q>, Q: Eq + Hash {
        let hash = self.hash(key);
        let segment_index = self.get_segment(hash);
        self.segments[segment_index].get_mut().unwrap().get(key)
    }

    pub fn with_capacity_and_hasher_and_concurrency_level(capacity: usize, hash_builder: B, concurrency_level: usize) -> Self {
        let concurrency_level = concurrency_level.next_power_of_two();
        let per_segment_capacity = (capacity / concurrency_level).next_power_of_two();
        let mut segments = Vec::with_capacity(concurrency_level);
        for _ in 0..concurrency_level {
            segments.push(RwLock::new(HashMap::with_capacity_and_hasher(per_segment_capacity, <B as Default>::default())))
        }
        ConcurrentHashMap{
            hash_builder,
            segments,
        }
    }

    fn hash<Q: ?Sized>(&self, key: &Q) -> u64 where K: Borrow<Q>, Q: Eq + Hash {
        let mut hasher = self.hash_builder.build_hasher();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn get_segment(&self, hash: u64) -> usize {
        let shift_size = (std::mem::size_of::<usize>() * 8) - self.segments.len().trailing_zeros() as usize;
        (hash as usize >> shift_size) & (self.segments.len() - 1)
    }
}

impl<K: Eq + Hash, V, B: BuildHasher + Default> Default for ConcurrentHashMap<K, V, B> {
    fn default() -> Self {
        ConcurrentHashMap::with_capacity_and_hasher_and_concurrency_level(DEFAULT_INITIAL_CAPACITY, Default::default(), DEFAULT_SEGMENT_COUNT)
    }
}

pub struct ConcurrentHashSet<K, B: BuildHasher = RandomState> {
    table: ConcurrentHashMap<K, (), B>,
}

impl<K: Eq + Hash> ConcurrentHashSet<K, RandomState> {
    pub fn new() -> Self {
        ConcurrentHashSet {
            table: ConcurrentHashMap::new()
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        ConcurrentHashSet{
            table: ConcurrentHashMap::with_capacity_and_hasher_and_concurrency_level(capacity, Default::default(), DEFAULT_SEGMENT_COUNT)
        }
    }

    pub fn with_capacity_and_concurrency_level(capacity: usize, concurrency_level: usize) -> Self {
        ConcurrentHashSet{
            table: ConcurrentHashMap::with_capacity_and_hasher_and_concurrency_level(capacity, Default::default(), concurrency_level)
        }        
    }
}

impl<K: Eq + Hash, B: BuildHasher + Default> ConcurrentHashSet<K, B> {
    #[inline]
    pub fn insert(&self, key: K) -> bool {
        self.table.insert(key, ()).is_none()
    }

    pub fn contains<Q: ?Sized>(&self, key: &Q) -> bool where K: Borrow<Q>, Q: Eq + Hash {
        self.table.contains(key)
    }

    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: B) -> Self {
        ConcurrentHashSet{
            table: ConcurrentHashMap::with_capacity_and_hasher_and_concurrency_level(capacity, hash_builder, DEFAULT_SEGMENT_COUNT)
        }
    }

    pub fn with_capacity_and_hasher_and_concurrency_level(capacity: usize, hash_builder: B, concurrency_level: usize) -> Self {
        ConcurrentHashSet{
            table: ConcurrentHashMap::with_capacity_and_hasher_and_concurrency_level(capacity, hash_builder, concurrency_level)
        }        
    }
}

impl<K: Eq + Hash, B: BuildHasher + Default> Default for ConcurrentHashSet<K, B> {
    fn default() -> Self {
        ConcurrentHashSet{
            table: ConcurrentHashMap::default()
        }
    }
}