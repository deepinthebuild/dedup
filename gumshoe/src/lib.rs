#![feature(allocator_api)]

#[cfg(test)]
extern crate byteorder;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;
#[cfg(test)]
extern crate rayon;

extern crate tempfile;
extern crate memmap;

use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering::*};
use std::ptr::{self, NonNull};
use std::mem;
use std::marker::{PhantomData, Send, Sync};
use std::hash::{BuildHasher, Hash, Hasher};
use std::collections::hash_map::RandomState;


mod allocators;
#[allow(unused_imports)]
use allocators::{LeakySlab, TempfileSlab};

const BRANCH_LEN: usize = 1 << SHIFT_SIZE;
const SHIFT_SIZE: usize = 4;
const MAX_LEVEL: usize = 64 / SHIFT_SIZE;

const TAG_SIZE: usize = 8;

const VALUE_TAG: usize = 1;
const BRANCH_TAG: usize = 2;
const LIST_TAG: usize = 3;

#[repr(align(8))]
struct ValueNode<K: Eq> {
    hash: u64,
    value: K,
}

impl<K: Eq> PartialEq for ValueNode<K> {
    fn eq(&self, rhs: &Self) -> bool {
        if self.hash != rhs.hash {
            false
        } else {
            self.value == rhs.value
        }
    }
}

impl<K: Eq> Eq for ValueNode<K> {}

impl<K: Eq> ValueNode<K> {
    #[inline]
    fn new(hash: u64, value: K) -> Self {
        ValueNode { hash, value }
    }

    #[inline]
    fn heaped(self) -> *mut Self {
        thread_local!(static ARENA: TempfileSlab = TempfileSlab::new());
        let p = unsafe { ARENA.with(|arena| arena.next()) }; 
        unsafe { ptr::write(p, self); }
        p
    }

    #[inline]
    unsafe fn from_heaped(heaped: *mut Self) -> Self {
        ptr::read(heaped)
    }
}

#[repr(align(8))]
struct ListNode<K: Eq> {
    next: AtomicPtr<ListNode<K>>,
    val_node: NonNull<ValueNode<K>>,
}

impl<K: Eq> ListNode<K> {
    #[inline]
    fn new(val_node: *mut ValueNode<K>) -> ListNode<K> {
        let next = AtomicPtr::from(ptr::null_mut());
        let val_node = NonNull::new(val_node).unwrap();
        ListNode { next, val_node }
    }

    #[inline]
    fn heaped(val: *mut ValueNode<K>) -> *mut ListNode<K> {
        let s = ListNode::new(val);
        Box::into_raw(Box::from(s))
    }

    #[inline]
    unsafe fn from_raw(raw: *mut ListNode<K>) -> ListNode<K> {
        let s = Box::from_raw(raw);
        *s
    }

    #[inline]
    fn consume(self) -> *mut ValueNode<K> {
        self.val_node.as_ptr()
    }

    #[inline]
    fn insert(&self, val: *mut ValueNode<K>) -> bool {
        unsafe {
            if (self.val_node).as_ref() == &*val {
                Box::from_raw(val);
                false
            } else {
                let next = self.next.load(Acquire);
                if next.is_null() {
                    let new = ListNode::heaped(val);
                    match self.next.compare_exchange(
                        ptr::null_mut(),
                        new,
                        AcqRel,
                        Acquire,
                    ) {
                        Ok(_) => true,
                        Err(existing) => {
                            let val = ListNode::from_raw(new).consume();
                            (&*existing).insert(val)
                        }
                    }
                } else {
                    (&*next).insert(val)
                }
            }
        }
    }

    #[inline]
    fn contains(&self, val: ValueNode<K>) -> bool {
        unsafe {
            if (self.val_node).as_ref() == &val {
                true
            } else {
                let next = self.next.load(Acquire);
                if next.is_null() {
                    false
                } else {
                    (&*next).contains(val)
                }
            }
        }
    }
}

enum EntryType<K: Eq> {
    Empty,
    Value(*mut ValueNode<K>),
    Branch(*mut Branch),
    List(*mut ListNode<K>),
}

impl<K: Eq> EntryType<K> {
    #[inline]
    fn from_entry(entry: usize) -> Self {
        if entry == 0 {
            EntryType::Empty
        } else {
            match entry & (TAG_SIZE - 1) {
                VALUE_TAG => EntryType::Value((entry ^ VALUE_TAG) as *mut ValueNode<K>),
                BRANCH_TAG => EntryType::Branch((entry ^ BRANCH_TAG) as *mut Branch),
                LIST_TAG => EntryType::List((entry ^ LIST_TAG) as *mut ListNode<K>),
                _ => panic!("Invalid tag encountered!"),
            }
        }
    }
}

#[repr(align(8))]
struct Branch {
    entries: [AtomicUsize; BRANCH_LEN],
}

impl Branch {
    fn new() -> Self {
        let entries = unsafe { mem::zeroed() };
        Branch { entries }
    }

    #[inline]
    fn heaped() -> *mut Branch {
        thread_local!(static ARENA: TempfileSlab = TempfileSlab::new());
        unsafe { ARENA.with(|arena| arena.next()) }
    }

    #[inline]
    fn insert<K: Eq>(&self, key: ValueNode<K>, level: usize) -> bool {
        let index = (key.hash >> (level * SHIFT_SIZE)) as usize % BRANCH_LEN;
        unsafe {
            let old = self.entries[index].load(Acquire);
            match EntryType::from_entry(old) {
                EntryType::Empty => {
                    let newval = key.heaped();
                    if self.entries[index]
                        .compare_exchange(0, newval as usize | VALUE_TAG, AcqRel, Acquire)
                        .is_err()
                    {
                        let key = ValueNode::from_heaped(newval);
                        self.insert(key, level)
                    } else {
                        true
                    }
                }
                EntryType::Branch(brnch) => (&*brnch).insert(key, level + 1),
                EntryType::List(list) => {
                    let key = key.heaped();
                    (&*list).insert(key)
                }
                EntryType::Value(old_val) => if key == *old_val {
                    false
                } else {
                    self.try_expand::<K>(old, index, level + 1);
                    self.insert(key, level)
                },
            }
        }
    }

    #[inline]
    fn try_expand<K: Eq>(&self, old_entry: usize, index: usize, level: usize) -> EntryType<K> {
        unsafe {
            match EntryType::from_entry(old_entry) {
                EntryType::Branch(brnch) => EntryType::Branch(brnch),
                EntryType::List(list) => EntryType::List(list),
                EntryType::Value(val) => {
                    if level >= MAX_LEVEL {
                        let leaf = ListNode::heaped(val);
                        let cmp = self.entries[index].compare_and_swap(
                            old_entry,
                            leaf as *const ListNode<K> as usize | LIST_TAG,
                            AcqRel,
                        );
                        if cmp == old_entry {
                            EntryType::List(leaf)
                        } else {
                            EntryType::from_entry(cmp)
                        }
                    } else {
                        let brnch = Branch::heaped();
                        (&mut *brnch).insert_uncontested(val, level);
                        let cmp = self.entries[index].compare_and_swap(
                            old_entry,
                            brnch as *const Branch as usize | BRANCH_TAG,
                            AcqRel,
                        );
                        if cmp == old_entry {
                            EntryType::Branch(brnch)
                        } else {
                            // If brnch is to be freed, do it here.
                            EntryType::from_entry(cmp)
                        }
                    }
                }
                EntryType::Empty => unreachable!(),
            }
        }
    }

    #[inline]
    fn insert_uncontested<K: Eq>(&mut self, val: *mut ValueNode<K>, level: usize) {
        unsafe {
            let index = ((*val).hash >> (level * SHIFT_SIZE)) as usize % BRANCH_LEN;
            let entry = self.entries[index].get_mut();
            *entry = val as usize | VALUE_TAG;
        }
    }

    #[inline]
    fn contains<K: Eq>(&self, val: ValueNode<K>, level: usize) -> bool {
        let index = (val.hash >> (level * SHIFT_SIZE)) as usize % BRANCH_LEN;
        let candidate = self.entries[index].load(Relaxed);
        unsafe {
            match EntryType::from_entry(candidate) {
                EntryType::Branch(brnch) => (&*brnch).contains(val, level + 1),
                EntryType::List(list) => (&*list).contains(val),
                EntryType::Value(old_val) => *old_val == val,
                EntryType::Empty => false,
            }
        }
    }
}

pub struct GumSet<K: Eq + Hash, B: BuildHasher = RandomState> {
    _marker: PhantomData<K>,
    branch_head: Box<Branch>,
    hash_builder: B,
}

impl<K: Eq + Hash> GumSet<K> {
    pub fn new() -> Self {
        GumSet::default()
    }
}

impl<'a, K: Eq + Hash + 'a, B: BuildHasher> GumSet<K, B> {
    pub fn insert(&'a self, key: K) -> bool {
        let hash = self.hash64(&key);
        let val = ValueNode::new(hash, key);
        self.branch_head.insert(val, 0)
    }

    pub fn contains(&'a self, key: K) -> bool {
        let hash = self.hash64(&key);
        let val = ValueNode::new(hash, key);
        self.branch_head.contains(val, 0)
    }

    #[inline]
    fn hash64(&self, key: &K) -> u64 {
        let mut hasher = self.hash_builder.build_hasher();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

impl<K: Eq + Hash, B: BuildHasher + Default> Default for GumSet<K, B> {
    fn default() -> Self {
        GumSet {
            _marker: PhantomData,
            branch_head: Box::new(Branch::new()),
            hash_builder: Default::default(),
        }
    }
}

unsafe impl<K: Hash + Eq + Sync, B: BuildHasher + Sync> Sync for GumSet<K, B> {}
unsafe impl<K: Hash + Eq + Send, B: BuildHasher + Send> Send for GumSet<K, B> {}


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
    fn gumset_insert_test() {
        let data = vec![5u8, 100u8, 200u8];
        let set = GumSet::<&[u8]>::new();
        set.insert(&data[..]);

        assert!(set.contains(&data[..]));
    }

    #[test]
    fn gumset_realloc_test() {
        let data: Vec<u8> = (0..200u8).into_iter().collect();
        let set = GumSet::new();
        for s in data.windows(2) {
            set.insert(s);
        }

        for s in data.windows(2) {
            assert!(set.contains(s));
        }
    }

    #[test]
    fn concurrent_inserts_test() {
        let set = Arc::new(GumSet::new());

        let handle1 = Arc::clone(&set);
        let child1 = thread::spawn(move || {
            for s in DATA1.windows(2) {
                handle1.insert(s);
            }
        });

        let handle2 = Arc::clone(&set);
        let child2 = thread::spawn(move || {
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
        let set = Arc::new(GumSet::<&'static [u8]>::new());
        let mut handles = Vec::with_capacity(16);

        for t in 0..16 {
            let setarc = Arc::clone(&set);
            handles.push(thread::spawn(move || {
                for s in DATA1.windows((t / 2) + 1) {
                    setarc.insert(s);
                }
            }));
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
        let set = GumSet::<&'static [u8]>::new();
        let results: Vec<&[u8]> = DATA3.par_windows(2).filter(|s| set.insert(s)).collect();
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
        let set = GumSet::new();
        let results: Vec<&[u8]> = DATA4.par_chunks(4).filter(|s| set.insert(s)).collect();
        assert_eq!(results.len(), 1_000_000);
    }

    #[test]
    fn detect_duplicates_test() {
        let set = GumSet::new();

        for s in DATA1.windows(3) {
            assert!(set.insert(s));
        }

        for s in DATA1.windows(3) {
            assert!(!set.insert(s));
        }
    }
}
