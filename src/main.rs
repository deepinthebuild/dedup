#[macro_use]
extern crate clap;
extern crate memchr;
extern crate memmap;
extern crate seahash;

use memmap::Mmap;

use args::Args;
use error::DedupError;
use buffer::UnsortedBufferDeduper;
use stream::UnsortedStreamDeduper;

use std::io::{self, BufWriter, Read};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::process;

mod buffer;
mod error;
mod args;
mod stream;

fn main() {
    match Args::parse().and_then(run) {
        Ok(_) => process::exit(0),
        Err(DedupError::ClosedPipe) => process::exit(0),
        Err(u) => {
            eprintln!("{}", u);
            process::exit(1);
        }
    };
}

fn run(args: Args) -> Result<u64, DedupError> {
    if args.input.is_some() {
        run_on_file(args)
    } else {
        run_on_stdin(args)
    }
}

fn run_on_file(args: Args) -> Result<u64, DedupError> {
    if args.mmap {
        let input = memmap_file(args.input.as_ref().unwrap())?;
        if let Some(ref p) = args.output {
            let output = OpenOptions::new().write(true).create(true).open(p)?;
            let output = BufWriter::new(output);
            let dedup = UnsortedBufferDeduper::new(&input, output, (&args).into());
            dedup.run()
        } else {
            let out = io::stdout();
            let output = out.lock();
            let dedup = UnsortedBufferDeduper::new(&input, output, args.into());
            dedup.run()
        }
    } else {
        let input = read_file_to_vec(args.input.as_ref().unwrap())?;
        if let Some(ref p) = args.output {
            let output = OpenOptions::new().write(true).create(true).open(p)?;
            let output = BufWriter::new(output);
            let dedup = UnsortedBufferDeduper::new(&input, output, (&args).into());
            dedup.run()
        } else {
            let out = io::stdout();
            let output = out.lock();
            let dedup = UnsortedBufferDeduper::new(&input, output, args.into());
            dedup.run()
        }
    }
}

fn run_on_stdin(args: Args) -> Result<u64, DedupError> {
    let _input = io::stdin();
    let input = _input.lock();

    if let Some(ref p) = args.output {
        let output = OpenOptions::new().write(true).open(p)?;
        let output = BufWriter::new(output);
        let dedup = UnsortedStreamDeduper::new(input, output, (&args).into());
        dedup.run()
    } else {
        let out = io::stdout();
        let output = out.lock();
        let dedup = UnsortedStreamDeduper::new(input, output, args.into());
        dedup.run()
    }
}

fn read_file_to_vec<T: AsRef<Path>>(target: T) -> Result<Vec<u8>, io::Error> {
    let mut file = File::open(target)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}
fn memmap_file<T: AsRef<Path>>(target: T) -> Result<Mmap, io::Error> {
    let file = File::open(target)?;
    unsafe { Mmap::map(&file) }
}
