#![feature(test, cfg_target_feature, stdsimd)]

extern crate faster;

use faster::prelude::*;

#[cfg(all(target_arch = "x86_64", all(not(target_feature = "avx"), target_feature = "sse2")))]
use std::arch::x86_64::_mm_movemask_epi8 as movemask;

#[cfg(all(target_arch = "x86_64", target_feature = "avx"))]
use std::arch::x86_64::_mm256_movemask_epi8 as movemask;

#[cfg(all(target_arch = "x86", target_feature = "sse2"))]
use std::arch::x86::_mm_movemask_epi8 as movemask;

use std::mem;

#[inline]
pub fn fastchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    let mut iter = haystack.simd_iter();

    while let Some(v) = iter.next_vector() {
        let b = v_to_byte_mask(v, needle);
        if b != 0 {
            return Some(iter.scalar_position() + b.trailing_zeros() as usize - u8s::WIDTH);
        }
    }

    while let Some(s) = iter.next() {
        if s == needle {
            return Some(iter.scalar_position() - 1);
        }
    }

    None
}

#[inline]
fn v_to_byte_mask(v: u8s, needle: u8) -> usize {
    unsafe {
        let v = v.eq(u8s(needle));
        movemask(mem::transmute(v)) as usize 
    }
}



#[cfg(test)]
mod tests {
    extern crate test;
    extern crate memchr;

    use self::test::Bencher;
    use self::memchr::memchr;

    use super::*;

    use std::iter;

    const LONG_PREFIX_LEN: usize = 500000;
    const SHORT_PREFIX_LEN: usize = u8s::WIDTH * 2 - 1;
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
        println!("{:?}", haystack);
        println!("{:?}", (&haystack[..]).simd_iter().next_vector());
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