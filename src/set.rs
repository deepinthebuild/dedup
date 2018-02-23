use fxhash::FxHasher;

use std::collections::HashSet;
use std::hash::BuildHasherDefault;

pub type Set<T> = HashSet<T, BuildHasherDefault<FxHasher>>;