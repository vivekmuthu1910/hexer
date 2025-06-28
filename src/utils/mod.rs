use std::path::{Component, PathBuf};
pub fn last_n_components(path: &PathBuf, n: usize) -> (usize, PathBuf) {
    let comps: Vec<Component> = path.components().rev().take(n).collect();
    (path.components().count(), comps.into_iter().rev().collect())
}
