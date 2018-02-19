use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use crossbeam_utils::scoped::{scope, ScopedJoinHandle};
use crossbeam_deque::{Deque, Steal};

use lumpy_chunks::LumpyChunks;
use fastchr::fastchr;

use args::Options;
use error::DedupError;
use set::ConcurrentSet;
use output::{LockableSink, TetheredBufWriter};

use std::io::{Write};

const WORK_CHUNK_SIZE: usize = 27;

pub struct BufferDeduper<'a> {
    buffer: &'a [u8],
    opts: Options,
    dup_store: ConcurrentSet<&'a [u8]>,
}

impl<'a> BufferDeduper<'a> {
    pub fn new<R: AsRef<[u8]>>(buffer: &'a R, opts: Options) -> Self {
        BufferDeduper {
            buffer: buffer.as_ref(),
            dup_store: ConcurrentSet::default(),
            opts,
        }
    }

    pub fn run(self) -> Result<Option<u64>, DedupError> {
        ThreadPoolBuilder::new()
            .num_threads(self.opts.num_threads)
            .build_global()
            .unwrap();
        let terminator = self.opts.terminator;
        let set = &self.dup_store;
        let repeated = self.opts.repeated;
        let sink = LockableSink::new(self.opts.output.as_ref())?;
        let mut out = TetheredBufWriter::new(&sink); 

        let output: Vec<&[u8]> = self.buffer
            .par_split(|p| *p == terminator)
            .filter(|s| set.insert(s) != repeated)
            .collect();

        let mut len = output.len();

        if !self.opts.line_count {
                for s in &output[..len - 1] {
                    out.write_all(s)?;
                    out.write_all(&[terminator])?;
                }

                if let Some(s) = output.last() {
                    if !s.is_empty() {
                        out.write_all(s)?;
                        out.write_all(&[terminator])?;
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

    pub fn run_parallel(self) -> Result<Option<u64>, DedupError> {
        let terminator = self.opts.terminator;
        let set = &self.dup_store;
        let work_deque = Deque::with_min_capacity(16);
        let sink = LockableSink::new(self.opts.output.as_ref())?;
        let mut worker_results: Vec<Result<u64, DedupError>> = Vec::new();
        for piece in self.buffer.lumpy_chunks(WORK_CHUNK_SIZE, |s| fastchr(terminator, s)) {
            work_deque.push(piece);
        }
        scope(|scope| {
            let mut worker_handles: Vec<ScopedJoinHandle<Result<u64, DedupError>>> = Vec::with_capacity(::num_cpus::get());
            for _ in 0..::num_cpus::get() {
                let s = work_deque.stealer();
                let sink_ref = &sink;
                worker_handles.push(scope.spawn(move || {
                    let mut local_count = 0;
                    let mut write_buf: Vec<u8> = Vec::with_capacity(1 << 23);
                    loop {
                        match s.steal() {
                            Steal::Empty => {
                                if !write_buf.is_empty() {
                                    sink_ref.lock_write_all(&write_buf)?;
                                }
                                return Ok(local_count);
                                },
                            Steal::Data(mut working_slice) => {
                                while let Some(needle_index) = fastchr(terminator, working_slice) {
                                    let (head, tail) = working_slice.split_at(needle_index);
                                    working_slice = tail.get(1..).unwrap_or(&[]);
                                    if set.insert(head) {
                                        if head.len() + write_buf.len() + 1 > write_buf.capacity() {
                                            sink_ref.lock_write_all(&write_buf)?;
                                            write_buf.clear();
                                        }
                                        if head.len() + 1 > write_buf.capacity() {
                                            let mut outlock = sink_ref.lock();
                                            outlock.write_all(head)?;
                                            outlock.write_all(&[terminator])?;
                                        } else {
                                            write_buf.write_all(head)?;
                                            write_buf.write_all(&[terminator])?;
                                        }
                                        local_count += 1;
                                    }
                                }

                                if !working_slice.is_empty() && set.insert(working_slice) {
                                    if working_slice.len() + write_buf.len() + 1 > write_buf.capacity() {
                                        sink_ref.lock_write_all(&write_buf)?;
                                        write_buf.clear();
                                    }

                                    if working_slice.len() + 1 > write_buf.capacity() {
                                        let mut outlock = sink_ref.lock();
                                        outlock.write_all(&write_buf)?;
                                        outlock.write_all(&[terminator])?;
                                    } else {
                                        write_buf.write_all(working_slice)?;
                                        write_buf.write_all(&[terminator])?;
                                    }
                                    local_count += 1;
                                }
                            },
                            Steal::Retry => continue,
                        }
                    }
                }));
            }

            worker_results = worker_handles.into_iter().map(|h| h.join().unwrap_or_else(|err| Err(err.into()))).collect(); 
        });
        
        let mut count = 0u64;
        for res in worker_results {
            count += res?;
        }
        Ok(Some(count))
    }
}

#[allow(dead_code)]
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

}
