#![allow(dead_code)]

use std::path::{PathBuf};

use args::{Options, Args};
use error::DedupError;
use output::LockableSink;


#[derive(Debug)]
enum WorkType<'a> {
    Mmap(&'a [u8]),
    ReadIn(Vec<u8>),
}

pub struct Manager {
    inputs: Vec<PathBuf>,
    output: LockableSink,
    opts: Options,

}

impl Manager {
    pub fn from_args(args: Args) -> Result<Self, DedupError> {
        let output = LockableSink::new(args.output.as_ref())?;
        let opts = Options::from(&args);
        let inputs = args.input.into_iter().collect();
        Ok(Manager{inputs, output, opts})
    }
}