#![allow(dead_code)]

extern crate memchr;
extern crate memmap;
#[macro_use] extern crate clap;

use clap::App;
use memmap::Mmap;

use error::DedupError;
use unsorted::UnsortedBufferDeduper;

use std::fs::File;
use std::path::Path;
use std::io::{stdout};

mod config;
mod unsorted;
mod error;

fn main() {
    let yml = load_yaml!("../cli.yml");
    let matches = App::from_yaml(yml).get_matches();

    if let Some(target) = matches.value_of("INPUT") {
        
        let input = memmap_file(target).unwrap();
        let out = stdout();
        let output = out.lock();
        let dedup = UnsortedBufferDeduper::new(&input, output);
        dedup.run().unwrap();
        
    }
}


fn memmap_file<T: AsRef<Path>>(target: T) -> Result<Mmap, std::io::Error> {
    let file = File::open(target)?;
    unsafe {Mmap::map(&file)}
}