use std::fmt::{Display, Error, Formatter};
use std::io;

#[derive(Debug)]
pub enum DedupError {
    ClosedPipe,
    ArgumentParseError(String),
    IO(io::Error),
}

impl Display for DedupError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match *self {
            DedupError::ClosedPipe => write!(f, "A closed pipe was encountered"),
            DedupError::Other => write!(f, "An unknown error has occurred"),
            DedupError::IO(ref i) => write!(f, "{}", i),
            DedupError::ArgumentParseError(ref s) => write!(f, "{}", s),
        }
    }
}

impl From<io::Error> for DedupError {
    fn from(src: io::Error) -> DedupError {
        if let io::ErrorKind::BrokenPipe = src.kind() {
            DedupError::ClosedPipe
        } else {
            DedupError::IO(src)
        }
    }
}
