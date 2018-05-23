extern crate fastchr;
extern crate memchr;
#[macro_use]
extern crate quickcheck;
extern crate rand;

use fastchr::{fastchr, Fastchr};
use memchr::{memchr, Memchr};
use rand::{ChaChaRng, Rng};

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

#[test]
fn memchr_iter_count_equivalence() {
    let random_data: Vec<u8> = 
        ChaChaRng::new_unseeded().gen_iter().take(LONG_PREFIX_LEN * 2).collect();
    assert_eq!(Memchr::new(NEEDLE, &random_data).count(), Fastchr::new(NEEDLE, &random_data).count())
}

#[test]
fn memchr_dense_iter_count_equivalence() {
    let dense_data: Vec<u8> = 
        iter::repeat(NEEDLE).take(ODD_PREFIX_LEN).collect();
    assert_eq!(Memchr::new(NEEDLE, &dense_data).count(), Fastchr::new(NEEDLE, &dense_data).count())
}

quickcheck!{
    fn qc_memchr_equivalence(needle: u8, haystack: Vec<u8>) -> bool {
        fastchr(needle, &haystack) == memchr(needle, &haystack)
    }

    fn qc_memchr_iter_equivalence(needle: u8, haystack: Vec<u8>) -> bool {
        let f = Fastchr::new(needle, &haystack);
        let m = Memchr::new(needle, &haystack);
        f.zip(m).all(|(f, m)| f == m)
    }
}
