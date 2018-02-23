#![feature(test, cfg_target_feature)]


extern crate faster;
extern crate stdsimd;


use faster::prelude::*;
use stdsimd::vendor::*;

#[inline]
pub fn fastchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    let mut iter = haystack.simd_iter();

    while let Some(v) = iter.next_vector() {
        let b = v_to_byte_mask(v, needle);
        if b != 0 {
            return Some(iter.scalar_position() + b.trailing_zeros() as usize);
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
        let n = u8s(needle);
        let n = v.eq(n);

        #[cfg(all(not(target_feature = "avx"), target_feature = "sse2"))]
        {_mm_movemask_epi8(n) as usize }
        
        #[cfg(target_feature = "avx")]
        {_mm256_movemask_epi8(n) as usize }
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
    const SHORT_PREFIX_LEN: usize = 5;
    const ODD_PREFIX_LEN: usize = 50003;

    fn generate_long_sample() -> Vec<u8> {
            iter::repeat(14u8).take(LONG_PREFIX_LEN)
            .chain(iter::once(70u8))
            .collect()
    }

    fn generate_short_sample() -> Vec<u8> {
            iter::repeat(14u8).take(SHORT_PREFIX_LEN)
            .chain(iter::once(70u8))
            .collect()
    }

    fn generate_odd_sample() -> Vec<u8> {
            iter::repeat(14u8).take(ODD_PREFIX_LEN)
            .chain(iter::once(70u8))
            .collect()
    }


    #[test]
    fn long_find_test() {
        let data = generate_long_sample();
        assert_eq!(Some(LONG_PREFIX_LEN), fastchr(70u8, &data));
    }

    #[test]
    fn short_find_test() {
        let data = generate_short_sample();
        assert_eq!(Some(SHORT_PREFIX_LEN), fastchr(70u8, &data));
    }

    #[test]
    fn odd_find_test() {
        let data = generate_odd_sample();
        assert_eq!(Some(ODD_PREFIX_LEN), fastchr(70u8, &data));
    }

    #[test]
    fn empty_find_test() {
        let data = Vec::new();
        assert_eq!(None, fastchr(70u8, &data));
    }

    #[bench]
    fn fastchr_bench(b: &mut Bencher) {
        let data = generate_long_sample();
        b.iter(|| fastchr(70u8, &data));
    }

    #[bench]
    fn memchr_bench(b: &mut Bencher) {
        let data = generate_long_sample();
        b.iter(|| memchr(70u8, &data));
    }

}