#![feature(test)]

extern crate fastchr;
extern crate memchr;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate criterion;
extern crate rand;

use criterion::Criterion;
use fastchr::{Fastchr, fastchr};
use memchr::{Memchr, memchr};
use rand::{ChaChaRng, Rng};

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

    static ref NEGATIVE_DATA: Vec<u8> = {
        iter::repeat(150u8).take(LONG_PREFIX_LEN).collect()
    };

    static ref RANDOM_DATA: Vec<u8> = {
        ChaChaRng::new_unseeded().gen_iter().take(LONG_PREFIX_LEN * 2).collect()
    };

    static ref DENSE_DATA: Vec<u8> = {
        iter::repeat(NEEDLE).take(ODD_PREFIX_LEN).collect()
    };
}

fn bench_fastchr(c: &mut Criterion) {
    c.bench_function("fastchr",
        |b| b.iter(|| assert_eq!(fastchr(NEEDLE, &DATA), Some(LONG_PREFIX_LEN)))
     );
}

fn bench_fastchr_negative(c: &mut Criterion) {
    c.bench_function("fastchr with negative test case",
        |b| b.iter(|| assert_eq!(fastchr(NEEDLE, &NEGATIVE_DATA), None))
     );
}

fn bench_memchr(c: &mut Criterion) {
    c.bench_function("memchr",
        |b| b.iter(|| assert_eq!(memchr(NEEDLE, &DATA), Some(LONG_PREFIX_LEN)))
    );
}

fn bench_memchr_negative(c: &mut Criterion) {
    c.bench_function("memchr with negative test case",
        |b| b.iter(|| assert_eq!(memchr(NEEDLE, &NEGATIVE_DATA), None))
    );
}

fn bench_stdlib_position(c: &mut Criterion) {
    c.bench_function("stdlib position iterator",
        |b| b.iter(|| assert_eq!(DATA.iter().position(|&byte| byte == NEEDLE), Some(LONG_PREFIX_LEN)))
    );
}

fn bench_stdlib_position_negative(c: &mut Criterion) {
    c.bench_function("stdlib position iterator with negative test case",
        |b| b.iter(|| assert_eq!(NEGATIVE_DATA.iter().position(|&byte| byte == NEEDLE), None))
    );
}

fn bench_fastchr_iter(c: &mut Criterion) {
    c.bench_function("fastchr iterator count",
        |b| b.iter(|| Fastchr::new(NEEDLE, &RANDOM_DATA).count())
     );
}

fn bench_dense_fastchr_iter(c: &mut Criterion) {
    c.bench_function("fastchr dense iterator count",
        |b| b.iter(|| Fastchr::new(NEEDLE, &DENSE_DATA).count())
     );
}

fn bench_memchr_iter(c: &mut Criterion) {
    c.bench_function("memchr iterator count",
        |b| b.iter(|| Memchr::new(NEEDLE, &RANDOM_DATA).count())
     );
}

fn bench_dense_memchr_iter(c: &mut Criterion) {
    c.bench_function("memchr dense iterator count",
        |b| b.iter(|| Memchr::new(NEEDLE, &DENSE_DATA).count())
     );
}

criterion_group!(benches, bench_fastchr, bench_fastchr_negative, bench_memchr, bench_memchr_negative, bench_stdlib_position, bench_stdlib_position_negative);
criterion_group!(iters, bench_fastchr_iter, bench_dense_fastchr_iter, bench_memchr_iter, bench_dense_memchr_iter);
criterion_main!(benches, iters);