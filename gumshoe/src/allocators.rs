#![allow(dead_code)]

use tempfile::tempfile;
use memmap::MmapMut;

use std::heap::{self, Alloc, Layout};
use std::cell::Cell;
use std::mem::{size_of, forget};

const DEFAULT_ALIGN: usize = 16;

pub(crate) struct LeakySlab {
    ptr: Cell<*mut u8>,
    index: Cell<usize>,
}

impl LeakySlab {
    /// Hopefully slabs are page aligned for some memory efficiency
    const SLAB_SIZE: usize = (1 << 21);

    #[inline]
    pub fn new() -> Self {
        let mut allocator = heap::Global;
        let layout = Layout::from_size_align(Self::SLAB_SIZE, DEFAULT_ALIGN).unwrap();
        let ptr = unsafe { allocator.alloc_zeroed(layout).unwrap() };
        LeakySlab {
            ptr: Cell::new(ptr.as_ptr() as *mut u8),
            index: Cell::new(0),
        }
    }

    #[inline]
    fn refresh(&self) {
        let mut allocator = heap::Global;
        let layout = Layout::from_size_align(Self::SLAB_SIZE, DEFAULT_ALIGN).unwrap();
        let ptr = unsafe { allocator.alloc_zeroed(layout).unwrap() };
        self.ptr.set(ptr.as_ptr() as *mut u8);
        self.index.set(0);
    }

    #[inline]
    pub unsafe fn next<T: Sized>(&self) -> *mut T {
        let i = self.index.get();
        let s = size_of::<T>();
        if i + s < Self::SLAB_SIZE {
            self.index.set(i + s);
            let ptr = self.ptr.get();
            ptr.offset((i + s) as isize) as *mut T
        } else {
            self.refresh();
            self.next()
        }
    }
}

pub(crate) struct TempfileSlab {
    ptr: Cell<*mut u8>,
    index: Cell<usize>,
}

impl TempfileSlab {
    /// Hopefully slabs are page aligned for some memory efficiency
    const SLAB_SIZE: usize = 1 << 26;

    pub fn new() -> Self {
        let ptr = Self::map_temp_file();

        TempfileSlab{
            ptr: Cell::new(ptr),
            index: Cell::new(0),
        }
    }

    fn map_temp_file() -> *mut u8 {
        let temp = tempfile().unwrap();
        temp.set_len(Self::SLAB_SIZE as u64).unwrap();
        let mut mmap = unsafe { MmapMut::map_mut(&temp).unwrap() };
        let ptr = mmap.as_mut_ptr();
        forget(mmap);
        forget(temp);
        ptr
    }
    #[inline]
    fn refresh(&self) {
        let ptr = Self::map_temp_file();
        self.ptr.set(ptr);
        self.index.set(0);
    }

    #[inline]
    pub unsafe fn next<T: Sized>(&self) -> *mut T {
        let i = self.index.get();
        let s = size_of::<T>();
        if i + s < Self::SLAB_SIZE {
            self.index.set(i + s);
            let ptr = self.ptr.get();
            ptr.offset((i + s) as isize) as *mut T
        } else {
            self.refresh();
            self.next()
        }
    }
}