#![feature(test)]

extern crate fastchr;
extern crate test;
extern crate memchr;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate criterion;

use criterion::Criterion;
use fastchr::fastchr;
use memchr::memchr;

use std::iter;

const LONG_PREFIX_LEN: usize = 500000;
const ODD_PREFIX_LEN: usize = 50003;
const NEEDLE: u8 = 70;

lazy_static!{
    static ref DATA: Vec<u8> = {
        iter::repeat(14u8).take(LONG_PREFIX_LEN)
        .chain(iter::once(NEEDLE))
        .chain(iter::repeat(200u8).take(ODD_PREFIX_LEN))
        .collect()
    };
}

fn bench_fastchr(c: &mut Criterion) {
    c.bench_function("fastchr",
        |b| b.iter(|| assert_eq!(fastchr(NEEDLE, &DATA), Some(LONG_PREFIX_LEN)))
     );
}

fn bench_memchr(c: &mut Criterion) {
    c.bench_function("memchr",
        |b| b.iter(|| assert_eq!(memchr(NEEDLE, &DATA), Some(LONG_PREFIX_LEN)))
    );
}

fn bench_iter(c: &mut Criterion) {
    c.bench_function("iterator",
        |b| b.iter(|| assert_eq!(DATA.iter().position(|&byte| byte == NEEDLE), Some(LONG_PREFIX_LEN)))
    );
}

criterion_group!(benches, bench_fastchr, bench_memchr, bench_iter);
criterion_main!(benches);