#[derive(Debug)]
pub enum DedupError {
    BrokenPipe,
    Other,
}