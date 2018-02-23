#[cfg(not(feature = "simd-accel"))]
use memchr::memchr;
#[cfg(feature = "simd-accel")]
use fastchr::fastchr as memchr;

use args::Options;
use error::DedupError;
use set::Set;

use std::io;
use std::default::Default;

pub struct BufferDeduper<'a, W: io::Write + 'a> {
    buffer: &'a [u8],
    opts: Options,
    out: W,
    dup_store: Set<&'a [u8]>,
}

impl<'a, W: io::Write + 'a> BufferDeduper<'a, W> {
    pub fn new<R: AsRef<[u8]>>(buffer: &'a R, output: W, opts: Options) -> Self {
        BufferDeduper {
            buffer: buffer.as_ref(),
            out: output,
            dup_store: Set::with_capacity_and_hasher(
                buffer.as_ref().len() / 50,
                Default::default(),
            ),
            opts,
        }
    }

    pub fn run(mut self) -> Result<u64, DedupError> {
        let delim = self.opts.delim;
        let mut count: u64 = 0;
        while let Some(u) = memchr(delim, self.buffer) {
            let (mut ele, rest) = self.buffer.split_at(u + 1);
            if self.dup_store.insert(ele) {
                self.out.write_all(ele)?;
            }
            self.buffer = rest;
            count += 1;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;
    static BREAKFAST: &str = "\
spam
ham
eggs
ham
ham eggs
eggs
ham
spam
";

    static BREAKFAST_DEDUP: &str = "\
spam
ham
eggs
ham eggs
";

    #[test]
    fn buf_breakfast_dedup() {
        let mut output: Vec<u8> = Vec::new();
        {
            let dedup = BufferDeduper::new(&BREAKFAST, &mut output, Options::default());
            dedup.run().unwrap();
        }
        assert_eq!(BREAKFAST_DEDUP, str::from_utf8(&output).unwrap());
    }
}
