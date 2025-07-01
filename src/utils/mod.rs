use std::path::{Component, Path, PathBuf};

pub fn last_n_components(path: &Path, n: usize) -> (usize, PathBuf) {
    let comps: Vec<Component> = path.components().rev().take(n).collect();
    (path.components().count(), comps.into_iter().rev().collect())
}

pub fn previous_power_of_two(x: u16) -> u16 {
    if x == 0 {
        0
    } else {
        1 << (15 - x.leading_zeros())
    }
}
