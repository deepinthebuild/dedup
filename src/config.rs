#[derive(Debug, Clone)]
pub struct Options {
    pub term: Terminator,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            term: Default::default(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Terminator {
    CRLF,
    Any(u8),
}

impl Default for Terminator {
    fn default() -> Self {
        Terminator::CRLF
    }
}
