use std::path::PathBuf;


pub trait RAID<const D: usize, const C: usize, const X: usize>: Sized {
    fn new(root_path: PathBuf) -> Self;
    fn add_data(&mut self, data: &[&[u8; X]; D], data_slice: usize);
    fn add_data_at(&mut self, data: &[u8; X], data_slice: usize, data_idx: usize);
    fn read_data(&self, data_slice: usize) -> [Box<[u8; X]>; D];
    fn read_data_at(&self, data_slice: usize, data_idx: usize) -> Box<[u8; X]>;
    fn destroy_devices(&self, dev_idxs: &[usize]);
    fn update_data(&self, data: &[u8; X], data_slice: usize, data_idx: usize);
    fn shutdown(self) {}
}
