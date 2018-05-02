#![feature(test, stdsimd)]

extern crate memchr;

use memchr::memchr;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
#[cfg(target_arch = "x86")]
use std::arch::x86::*;

use std::mem;

#[derive(Debug, Clone)]
pub struct Fastchr<'a> {
    needle: u8,
    haystack: &'a [u8],
    position: usize,
}

impl<'a> Fastchr<'a> {
    pub fn new(needle: u8, haystack: &'a [u8]) -> Self {
        Fastchr{
            needle,
            haystack,
            position: 0,
        }
    }
}

impl<'a> Iterator for Fastchr<'a> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        fastchr(self.needle, self.haystack).map(
            move |new_index| {
                self.haystack = self.haystack.split_at(new_index + 1).1;
                let found_pos = self.position + new_index;
                self.position = found_pos + 1;
                found_pos
            }
        )
    }
}

#[derive(Debug, Clone)]
pub struct FastchrSplit<'a> {
    needle: u8,
    haystack: &'a [u8],
}

impl<'a> FastchrSplit<'a> {
    pub fn new(needle: u8, haystack: &'a [u8]) -> Self {
        FastchrSplit{
            needle,
            haystack,
        }
    }
}

impl<'a> Iterator for FastchrSplit<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<Self::Item> {
        fastchr(self.needle, self.haystack).map(
            move |new_index| {
                let (split, haystack) = self.haystack.split_at(new_index + 1);
                self.haystack = haystack;
                split
            }
        )
    }
}


#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
#[inline]
pub fn fastchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    memchr(needle, haystack)
}

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
    const LANE_WIDTH: usize = 16;

    let needle = needle as i8;
    let haystack: &[i8] = mem::transmute(haystack);
    let ptr = haystack.as_ptr() as usize;
    let mut index: usize = 0;

    // Read a 16 byte lane at a time and check if any bytes are equal to the needle.
    let wide_needle = _mm_set1_epi8(needle);

    while index + LANE_WIDTH < haystack.len() {
        let hay = _mm_loadu_si128((ptr + index) as *const __m128i);
        let hay_cmp = _mm_cmpeq_epi8(hay, wide_needle);
        let hay_cmp_mask = _mm_movemask_epi8(hay_cmp) as usize;
        if hay_cmp_mask != 0 {
            return Some(index + hay_cmp_mask.trailing_zeros() as usize)
        }
        index += LANE_WIDTH;
    }
    
    // If there are bytes left over that don't fill a SIMD register, search them individually.
    while index < haystack.len() {
        if needle == *haystack.get_unchecked(index) {
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
    const LANE_WIDTH: usize = 32;

    let needle = needle as i8;
    let haystack: &[i8] = mem::transmute(haystack);
    let mut index = 0;
    let ptr = haystack.as_ptr() as usize;

    // align ptr to 32 bytes
    while (ptr + index) % 32 != 0 && index < haystack.len() {
        if needle == *haystack.get_unchecked(index) {
            return Some(index)
        } else {
            index += 1;
        }
    }

    // Read a 32 byte lane at a time and check if any are equal to the needle.
    let wide_needle = _mm256_set1_epi8(needle);

    while index + LANE_WIDTH < haystack.len() {
        let hay = _mm256_load_si256((ptr + index) as *const __m256i);
        let hay_cmp = _mm256_cmpeq_epi8(hay, wide_needle);
        let hay_cmp_mask = _mm256_movemask_epi8(hay_cmp) as usize;
        if hay_cmp_mask != 0 {
            return Some(index + hay_cmp_mask.trailing_zeros() as usize)
        }
        index += LANE_WIDTH;
    }
    
    // If there are bytes left over that don't fill a SIMD register, search them individually.
    while index < haystack.len() {
        if needle == *haystack.get_unchecked(index) {
            return Some(index)
        } else {
            index += 1;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    extern crate test;

    use self::test::Bencher;

    use super::*;

    use std::iter;

    const LONG_PREFIX_LEN: usize = 500000;
    const SHORT_PREFIX_LEN: usize = 16 * 2 - 1;
    const ODD_PREFIX_LEN: usize = 50003;
    const BURIED_SAMPLE_LEN: usize = 241;
    const NEEDLE: u8 = 70;

    fn generate_long_sample() -> Vec<u8> {
            iter::repeat(14u8).take(LONG_PREFIX_LEN)
            .chain(iter::once(NEEDLE))
            .chain(iter::repeat(200u8).take(ODD_PREFIX_LEN))
            .collect()
    }

    fn generate_short_sample() -> Vec<u8> {
            iter::repeat(14u8).take(SHORT_PREFIX_LEN)
            .chain(iter::once(NEEDLE))
            .collect()
    }

    fn generate_odd_sample() -> Vec<u8> {
            iter::repeat(14u8).take(ODD_PREFIX_LEN)
            .chain(iter::once(NEEDLE))
            .collect()
    }
    fn generate_long_negative_sample() -> Vec<u8> {
        iter::repeat(231).take(ODD_PREFIX_LEN)
        .collect()
    }

    fn generate_buried_sample() -> Vec<u8> {
        iter::repeat(231u8).take(BURIED_SAMPLE_LEN)
        .chain(iter::once(NEEDLE))
        .chain(iter::repeat(231u8).take(3))
        .chain(iter::once(NEEDLE))
        .chain(iter::repeat(231u8).take(ODD_PREFIX_LEN))
        .collect()
    }

    #[test]
    fn buried_sample_test() {
        let data = generate_buried_sample();
        assert_eq!(Some(BURIED_SAMPLE_LEN), fastchr(NEEDLE, &data));
    }

    #[test]
    fn negative_sample_test() {
        let data = generate_long_negative_sample();
        assert_eq!(None, fastchr(NEEDLE, &data));
    }
    
    #[test]
    fn qbf_test() {
        let haystack = b"the quick brown fox";
        assert_eq!(Some(8), fastchr(b'k', haystack));
    }

    #[test]
    fn memchr_odd_compat_test() {
        let data = generate_odd_sample();
        assert_eq!(memchr(NEEDLE, &data), fastchr(NEEDLE, &data));
    }

    #[test]
    fn memchr_short_compat_test() {
        let data = generate_short_sample();
        assert_eq!(memchr(NEEDLE, &data), fastchr(NEEDLE, &data));
    }

    #[test]
    fn long_find_test() {
        let data = generate_long_sample();
        assert_eq!(Some(LONG_PREFIX_LEN), fastchr(NEEDLE, &data));
    }

    #[test]
    fn short_find_test() {
        let data = generate_short_sample();
        assert_eq!(Some(SHORT_PREFIX_LEN), fastchr(NEEDLE, &data));
    }

    #[test]
    fn odd_find_test() {
        let data = generate_odd_sample();
        assert_eq!(Some(ODD_PREFIX_LEN), fastchr(NEEDLE, &data));
    }

    #[test]
    fn empty_find_test() {
        let data = Vec::new();
        assert_eq!(None, fastchr(NEEDLE, &data));
    }

    #[bench]
    fn fastchr_bench(b: &mut Bencher) {
        let data = generate_long_sample();
        b.iter(|| fastchr(NEEDLE, &data));
    }

    #[bench]
    fn memchr_bench(b: &mut Bencher) {
        let data = generate_long_sample();
        b.iter(|| memchr(NEEDLE, &data));
    }

}