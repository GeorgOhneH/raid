use crate::galois::Galois;
use std::path::PathBuf;

pub trait RAID<const D: usize, const C: usize, const X: usize> {
    fn new(root_path: PathBuf) -> Self;
    fn add_data(&mut self, data: &[[u8; X]; D]) -> usize;
    fn read_data(&self, data_slice: usize) -> [[u8; X]; D];
    fn destroy_devices(&self, dev_idxs: &[usize]);
    fn update_data(&self, data: &[u8; X], data_slice: usize, data_idx: usize);
}
