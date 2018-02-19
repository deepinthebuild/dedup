use gumshoe::GumSet;

use fxhash::FxHasher;
use seahash::SeaHasher;

use std::collections::HashSet;
use std::hash::BuildHasherDefault;

pub type Set<T> = HashSet<T, BuildHasherDefault<FxHasher>>;
pub type ConcurrentSet<K> = GumSet<K, BuildHasherDefault<SeaHasher>>;
