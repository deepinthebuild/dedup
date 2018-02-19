use args::Options;
use error::DedupError;
use set::Set;

use std::io;


pub struct UnsortedStreamDeduper<R: io::BufRead, W: io::Write> {
    input: R,
    opts: Options,
    out: W,
    dup_store: Set<Vec<u8>>,
}

impl<R: io::BufRead, W: io::Write> UnsortedStreamDeduper<R, W> {
    pub fn new(input: R, output: W, options: Options) -> Self {
        UnsortedStreamDeduper {
            input,
            opts: options,
            out: output,
            dup_store: Set::with_capacity_and_hasher(1024, Default::default()),
        }
    }

    pub fn run(mut self) -> Result<Option<u64>, DedupError> {
        let terminator = self.opts.terminator;
        let mut count: u64 = 0;

        loop {
            let mut buf = Vec::new();
            self.input.read_until(terminator, &mut buf)?;
            if buf.is_empty() {
                if self.opts.line_count {
                    return Ok(Some(count));
                } else {
                    return Ok(None)
                }
            }

            if self.opts.line_count {
                if self.dup_store.insert(buf) {
                    count += 1;
                }
            } else {
                if !self.dup_store.contains(&buf) {
                    self.out.write_all(&buf)?;
                    self.dup_store.insert(buf);
            }
            }

        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;
    use std::io::BufReader;

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
    fn stream_breakfast_dedup() {
        let mut output: Vec<u8> = Vec::new();
        let reader = BufReader::new(BREAKFAST.as_bytes());
        {
            let dedup = UnsortedStreamDeduper::new(reader, &mut output, Options::default());
            dedup.run().unwrap();
        }
        assert_eq!(BREAKFAST_DEDUP, str::from_utf8(&output).unwrap());
    }
}
