use memchr::memchr;

use config::{Options, Terminator};
use error::DedupError;

use std::io;
use std::default::Default;
use std::collections::HashSet;

pub struct UnsortedBufferDeduper<'a, W: io::Write + 'a> {
    buffer: &'a [u8],
    opts: Options,
    out_stream: W,
    dup_store: HashSet<&'a [u8]>,
}

impl<'a, W: io::Write + 'a> UnsortedBufferDeduper<'a, W> {
    pub fn new<R: AsRef<[u8]>>(buffer: &'a R, out_stream: W) -> Self {
        UnsortedBufferDeduper {
            buffer: buffer.as_ref(),
            out_stream,
            dup_store: Default::default(),
            opts: Default::default(),
        }
    }

    pub fn run(mut self) -> Result<u64, DedupError> {
        let mut crlf = false;
        let term = if let Terminator::Any(b) = self.opts.term {
            b
        } else {
            crlf = true;
            b'\n'
        };
        let mut count: u64 = 0;
        while let Some(u) = memchr(term, self.buffer) {
            let (mut ele, rest) = self.buffer.split_at(u);
            if crlf {
                if let Some(&b'\r') = ele.last() {
                    ele = &ele[..ele.len() - 1];
                }
            }
            if self.dup_store.insert(ele) {
                self.out_stream.write_all(ele).unwrap();
                self.out_stream.write_all(&[term]).unwrap();
            }
            self.buffer = &rest[1..];
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
    fn breakfast_dedup() {
        let mut output: Vec<u8> = Vec::new();
        {
            let dedup = UnsortedBufferDeduper::new(&BREAKFAST, &mut output);
            dedup.run();
        }
        assert_eq!(BREAKFAST_DEDUP, str::from_utf8(&output).unwrap());
    }
}
