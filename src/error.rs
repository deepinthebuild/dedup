use std::fmt::{Display, Error, Formatter};
use std::io;

#[derive(Debug)]
pub enum DedupError {
    ClosedPipe,
    ArgumentParseError(String),
    IO(io::Error),
    UnknownError(Box<::std::any::Any + ::std::marker::Send>)
}

impl Display for DedupError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match *self {
            DedupError::ClosedPipe => write!(f, "A closed pipe was encountered"),
            DedupError::IO(ref i) => write!(f, "{}", i),
            DedupError::ArgumentParseError(ref s) => write!(f, "{}", s),
            DedupError::UnknownError(ref boxed_err) => write!(f, "{:?}", boxed_err),

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

impl From<Box<::std::any::Any + ::std::marker::Send>> for DedupError {
    fn from(src: Box<::std::any::Any + ::std::marker::Send>) -> DedupError {
        DedupError::UnknownError(src)
    }
}