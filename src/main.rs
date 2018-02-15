#![allow(dead_code)]

extern crate memchr;
extern crate memmap;
#[macro_use] extern crate clap;

use memmap::Mmap;

use args::Args;
use error::DedupError;
use unsorted::UnsortedBufferDeduper;

use std::io::{self, BufWriter};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::process;

mod unsorted;
mod error;
mod args;

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
        let input = memmap_file(args.input.unwrap())?;
        if let Some(p) = args.output {
            let output = OpenOptions::new().write(true).open(p)?;
            let output = BufWriter::new(output);
            let dedup = UnsortedBufferDeduper::new(&input, output);
            dedup.run()
        } else {
            let out = io::stdout();
            let output = out.lock();
            let dedup = UnsortedBufferDeduper::new(&input, output);
            dedup.run()
        }
    } else {
        unimplemented!()
    }
}

fn run_on_stdin(args: Args) -> Result<u64, DedupError> {
    unimplemented!()
}

fn memmap_file<T: AsRef<Path>>(target: T) -> Result<Mmap, io::Error> {
    let file = File::open(target)?;
    unsafe {Mmap::map(&file)}
}