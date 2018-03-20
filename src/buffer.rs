/*
#[cfg(not(feature = "simd-accel"))]
use memchr::memchr;
#[cfg(feature = "simd-accel")]
use fastchr::fastchr as memchr;
*/
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;

use args::Options;
use error::DedupError;
use set::ConcurrentSet;

use std::io;

pub struct BufferDeduper<'a, W: io::Write + 'a> {
    buffer: &'a [u8],
    opts: Options,
    out: W,
    dup_store: ConcurrentSet,
}

impl<'a, W: io::Write + 'a> BufferDeduper<'a, W> {
    pub fn new<R: AsRef<[u8]>>(buffer: &'a R, output: W, opts: Options) -> Self {
        BufferDeduper {
            buffer: buffer.as_ref(),
            out: output,
            dup_store: ConcurrentSet::with_capacity(buffer.as_ref().len() / 256),
            opts,
        }
    }

    pub fn run(mut self) -> Result<Option<u64>, DedupError> {
        ThreadPoolBuilder::new()
            .num_threads(self.opts.num_threads)
            .build_global()
            .unwrap();
        let terminator = self.opts.terminator;
        let set = &self.dup_store;
        let repeated = self.opts.repeated;

        let output: Vec<&[u8]> = self.buffer.par_split(|p| *p == terminator)
            .filter(|s| set.insert(s) != repeated)
            .collect();
        

        let mut len = output.len();

        if !self.opts.line_count {
            for s in &output[..len - 1] {
                self.out.write_all(s)?;
                self.out.write_all(&[terminator])?;
            }

            if let Some(s) = output.last() {
                if !s.is_empty() {
                    self.out.write_all(s)?;
                    self.out.write_all(&[terminator])?;
                }
            }
            Ok(None)
        } else {
            if let Some(s) = output.last() {
                if s.is_empty() {
                    len -= 1;
                }
            }
            Ok(Some(len as u64))
        }
        
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
    #[ignore]
    fn buf_breakfast_dedup() {
        let mut output: Vec<u8> = Vec::new();
        {
            let dedup = BufferDeduper::new(&BREAKFAST, &mut output, Options::default());
            dedup.run().unwrap();
        }
        assert_eq!(BREAKFAST_DEDUP, str::from_utf8(&output).unwrap());
    }
}
