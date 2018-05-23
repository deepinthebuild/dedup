//! This crate provides one function: `fastchr`, which very quickly finds the first occurrence of a given byte in a slice.
//! `fastchr` is implemented using SIMD intrinsics and runtime CPU feature detection so it will always use the fastest method available
//! on a platform. If SIMD features are not available, `fastchr` falls back to using `memchr`.

#![warn(missing_docs)]
#![feature(stdsimd)]

extern crate memchr;

use memchr::memchr;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(target_arch = "x86")]
use std::arch::x86::*;

use std::mem;
use std::iter::FusedIterator;

const AVX_LANE_WIDTH: usize = 32;
const SSE_LANE_WIDTH: usize = 16;

/// An iterator for byte positions using `fastchr`.
/// 
/// This struct is created by [`Fastchr::new`].
#[derive(Debug, Clone)]
pub struct Fastchr<'a> {
    needle: u8,
    haystack: &'a [u8],
    position: usize,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    read_size: u8,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    detect_mask: u64,
}

impl<'a> Fastchr<'a> {
    /// Creates a new iterator that yields all positions of needle in haystack.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use fastchr::Fastchr;
    /// 
    /// let slice = [100, 50, 70, 50, 100, 100, 50];
    /// let mut iter = Fastchr::new(50, &slice[..]);
    /// 
    /// assert_eq!(iter.next().unwrap(), 1);
    /// assert_eq!(iter.next().unwrap(), 3);
    /// assert_eq!(iter.next().unwrap(), 6);
    /// assert!(iter.next().is_none());
    /// ```
    pub fn new(needle: u8, haystack: &[u8]) -> Fastchr {
        Fastchr{
            needle,
            haystack,
            position: 0,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            read_size: 0,
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            detect_mask: 0,
        }
    }

    fn read_to_mask(&mut self) {
        if is_x86_feature_detected!("avx2") {
            unsafe { self.avx_read_to_mask() }
        } else if is_x86_feature_detected!("sse2") {
            unsafe { self.sse_read_to_mask() }
        } else {
            self.fallback_read_to_mask()
        }
    }

    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn avx_read_to_mask(&mut self) {
        let needle = self.needle as i8;
        let mut read_size = 0;
        let ptr = self.haystack.as_ptr() as usize;

        while (ptr + read_size) % AVX_LANE_WIDTH != 0 && self.position + read_size < self.haystack.len() {
            if needle == *self.haystack.get_unchecked(self.position + read_size) as i8 {
                self.detect_mask |= 1 << read_size;
            }
            read_size += 1;
        }

        if read_size > 0 {
            self.position += read_size;
            self.read_size = read_size as u8;
            return
        }

        let wide_needle = _mm256_set1_epi8(needle);
        while self.position + AVX_LANE_WIDTH <= self.haystack.len() {
            let hay = _mm256_load_si256((ptr + self.position) as *const __m256i);
            let hay_cmp = _mm256_cmpeq_epi8(hay, wide_needle);
            let hay_cmp_mask = _mm256_movemask_epi8(hay_cmp) as u64;
            if hay_cmp_mask > 0 {
                self.read_size = AVX_LANE_WIDTH as u8;
                self.position += AVX_LANE_WIDTH;
                self.detect_mask = hay_cmp_mask;
                return
            }
            self.position += AVX_LANE_WIDTH
        }

        let mut read_size = 0;
        while self.position + read_size < self.haystack.len() {
            if needle == *self.haystack.get_unchecked(self.position + read_size) as i8 {
                self.detect_mask |= 1 << read_size;
            }
            read_size += 1;
        }
        self.position += read_size;
        self.read_size = read_size as u8;
    }

    #[inline]
    #[target_feature(enable = "sse2")]
    unsafe fn sse_read_to_mask(&mut self) {
        let needle = self.needle as i8;
        let ptr = self.haystack.as_ptr() as usize;

        let wide_needle = _mm_set1_epi8(needle);

        while self.position + SSE_LANE_WIDTH <= self.haystack.len() {
            let hay = _mm_loadu_si128((ptr + self.position) as *const __m128i);
            let hay_cmp = _mm_cmpeq_epi8(hay, wide_needle);
            let hay_cmp_mask = _mm_movemask_epi8(hay_cmp) as u64;
            if hay_cmp_mask > 0 {
                self.read_size = SSE_LANE_WIDTH as u8;
                self.position += SSE_LANE_WIDTH;
                self.detect_mask = hay_cmp_mask;
                return
            }
            self.position += SSE_LANE_WIDTH;
        }

        let mut read_size = 0;
        while self.position + read_size < self.haystack.len() {
            if needle == *self.haystack.get_unchecked(self.position + read_size) as i8 {
                self.detect_mask |= 1 << read_size;
            }
            read_size += 1;
        }
        self.position += read_size;
        self.read_size = read_size as u8;
    }

    fn fallback_read_to_mask(&mut self) {
        if let Some(u) = memchr(self.needle, &self.haystack[self.position..]) {
            self.position += u;
            self.read_size = 1;
        } else {
            self.position = self.haystack.len();
        }
    }
}

impl<'a> Iterator for Fastchr<'a> {
    type Item = usize;

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.detect_mask > 0 {
            // mask_pos is the index (starting from 0) of the lowest set bit
            let mask_pos = self.detect_mask.trailing_zeros() as usize;
            // This clears the lowest set bit in the mask
            self.detect_mask &= self.detect_mask - 1;
            Some(mask_pos + self.position - self.read_size as usize)
        } else if self.position >= self.haystack.len() {
            None
        } else {
            self.read_to_mask();
            self.next()
        }
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(x) = fastchr(self.needle, self.haystack) {
            self.haystack = self.haystack.split_at(x + 1).1;
            let found_pos = self.position + new_index;
            self.position = found_pos + 1;
            Some(found_pos)
        } else {
            self.haystack = &[];
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.haystack.len()))
    }
}

impl<'a> FusedIterator for Fastchr<'a> {}

/// An iterator of subslices separated by a specified byte. The separator byte is not included in the subslices.
/// 
/// This struct is created by [`FastchrSplit::new`]
#[derive(Debug, Clone)]
pub struct FastchrSplit<'a> {
    needle: u8,
    haystack: &'a [u8],
    finished: bool,
}

impl<'a> FastchrSplit<'a> {
    /// Returns an iterator over subslices of `haystack` separated by `needle`. `needle` is not contained in the subslices.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use fastchr::FastchrSplit;
    /// 
    /// let slice = [10, 40, 33, 20];
    /// let mut iter = FastchrSplit::new(40, &slice[..]);
    /// 
    /// assert_eq!(iter.next().unwrap(), &[10]);
    /// assert_eq!(iter.next().unwrap(), &[33, 20]);
    /// assert!(iter.next().is_none());
    /// ```
    /// 
    /// ```
    /// use fastchr::FastchrSplit;
    /// 
    /// let slice = [100, 50, 70, 50, 100, 100, 50];
    /// let mut iter = FastchrSplit::new(50, &slice[..]);
    /// 
    /// assert_eq!(iter.next().unwrap(), &[100]);
    /// assert_eq!(iter.next().unwrap(), &[70]);
    /// assert_eq!(iter.next().unwrap(), &[100, 100]);
    /// assert_eq!(iter.next().unwrap(), &[]);
    /// assert!(iter.next().is_none());
    /// ```
    pub fn new(needle: u8, haystack: &[u8]) -> FastchrSplit {
        FastchrSplit{
            needle,
            haystack,
            finished: false,
        }
    }
}

impl<'a> Iterator for FastchrSplit<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {   
        if self.finished {
            None
        } else {
            match fastchr(self.needle, self.haystack) {
                None => {
                    self.finished = true;
                    Some(mem::replace(&mut self.haystack, &[]))},
                Some(i) => {
                    let out = Some(&self.haystack[..i]);
                    self.haystack = &self.haystack[i + 1..];
                    out
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.finished {
            (0, Some(0))
        } else {
            (1, Some(self.haystack.len() + 1))
        }
    }
}

impl<'a> FusedIterator for FastchrSplit<'a> {}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
#[inline]
pub fn fastchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    memchr(needle, haystack)
}

/// Returns the index corresponding to the first occurrence of `needle` in `haystack`, or `None` if one is not found.
/// 
/// `fastchr` is implemented using SIMD intrinsics and run-time CPU feature detection, so it is often faster than `memchr`
/// due to being able to take advantage of available SIMD features at run-time.
/// 
/// # Example
/// 
/// ```
/// use fastchr::fastchr;
/// 
/// let haystack = b"the quick brown fox jumps over the lazy dog";
/// assert_eq!(fastchr(b'k', haystack), Some(8));
/// assert_eq!(fastchr(b'o', haystack), Some(12));
/// assert_eq!(fastchr(b'!', haystack), None);
/// ```
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline]
pub fn fastchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    if is_x86_feature_detected!("avx2") {
        unsafe { avx_fastchr(needle, haystack) }
    } else if is_x86_feature_detected!("sse2") {
        unsafe { sse_fastchr(needle, haystack) }
    } else {
        memchr(needle, haystack)
    }
}

#[inline]
#[target_feature(enable = "sse2")]
unsafe fn sse_fastchr(needle: u8, haystack: &[u8]) -> Option<usize> {

    let needle = needle as i8;
    let ptr = haystack.as_ptr() as usize;
    let mut index: usize = 0;

    // Read a 16 byte lane at a time and check if any bytes are equal to the needle.
    let wide_needle = _mm_set1_epi8(needle);

    while index + SSE_LANE_WIDTH <= haystack.len() {
        let hay = _mm_loadu_si128((ptr + index) as *const __m128i);
        let hay_cmp = _mm_cmpeq_epi8(hay, wide_needle);
        let hay_cmp_mask = _mm_movemask_epi8(hay_cmp) as usize;
        if hay_cmp_mask != 0 {
            return Some(index + hay_cmp_mask.trailing_zeros() as usize)
        }
        index += SSE_LANE_WIDTH;
    }
    
    // If there are bytes left over that don't fill a SIMD register, search them individually.
    while index < haystack.len() {
        if needle == *haystack.get_unchecked(index) as i8 {
            return Some(index)
        } else {
            index += 1;
        }
    }

    None
}

#[inline]
#[target_feature(enable = "avx2")]
unsafe fn avx_fastchr(needle: u8, haystack: &[u8]) -> Option<usize> {

    let needle = needle as i8;
    let mut index = 0;
    let ptr = haystack.as_ptr() as usize;

    // align ptr to 32 bytes
    while (ptr + index) % AVX_LANE_WIDTH != 0 && index < haystack.len() {
        if needle == *haystack.get_unchecked(index) as i8 {
            return Some(index)
        } else {
            index += 1;
        }
    }

    // Read a 32 byte lane at a time and check if any are equal to the needle.
    let wide_needle = _mm256_set1_epi8(needle);

    while index + AVX_LANE_WIDTH <= haystack.len() {
        let hay = _mm256_load_si256((ptr + index) as *const __m256i);
        let hay_cmp = _mm256_cmpeq_epi8(hay, wide_needle);
        let hay_cmp_mask = _mm256_movemask_epi8(hay_cmp) as u32;
        if hay_cmp_mask > 0 {
            return Some(index + hay_cmp_mask.trailing_zeros() as usize)
        }
        index += AVX_LANE_WIDTH;
    }
    
    // If there are bytes left over that don't fill a SIMD register, search them individually.
    while index < haystack.len() {
        if needle == *haystack.get_unchecked(index) as i8 {
            return Some(index)
        } else {
            index += 1;
        }
    }

    None
}
