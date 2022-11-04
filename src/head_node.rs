use std::path::{Path, PathBuf};
use std::fs::create_dir;
use crate::galois::{Galois, GaloisSlice};
use crate::matrix::Matrix;
use crate::matrix_slice::MatrixSlice;
use std::fs;
use std::prelude::rust_2021::TryInto;

pub struct HeadNode<const D: usize, const C: usize, const X: usize>
    where
        [(); D + C]:
{
    root_path: PathBuf,
    data_slices: usize,
    vandermonde: MatrixSlice::<C, D, X>,
    paths: [PathBuf; D + C],
}

impl<const D: usize, const C: usize, const X: usize> HeadNode<D, C, X>
    where
        [(); D + C]:
{
    pub fn new(path: PathBuf) -> Self {
        let paths = core::array::from_fn(|i| path.join(format!("device{i}")));
        for path in &paths {
            create_dir(path).unwrap()
        }

        Self {
            root_path: path,
            data_slices: 0,
            vandermonde: MatrixSlice::<C, D, X>::vandermonde(),
            paths,
        }
    }

    fn folder_id(data_slices: usize, data_idx: usize) -> usize {
        (data_idx + data_slices) % (D + C)
    }

    fn data_name(data_slices: usize, data_idx: usize) -> String {
        format!("{}_{}.data", data_slices, data_idx)
    }

    fn data_file(&self, data_slices: usize, data_idx: usize) -> PathBuf {
        let folder_path = &self.paths[Self::folder_id(data_slices, data_idx)];
        let name = Self::data_name(data_slices, data_idx);
        folder_path.join(name)
    }

    fn checksum_name(data_slices: usize, check_idx: usize) -> String {
        format!("{}_{}.data", data_slices, check_idx)
    }

    fn checksum_file(&self, data_slices: usize, check_idx: usize) -> PathBuf {
        let folder_path = &self.paths[Self::folder_id(data_slices, D+check_idx)];
        let name = Self::checksum_name(data_slices, check_idx);
        folder_path.join(name)
    }

    pub fn add_data(&mut self, data: &[GaloisSlice::<X>; D]) -> usize {
        let checksum = self.vandermonde.mul_vec(data);

        for d_idx in 0..D {
            let file_path = self.data_file(self.data_slices, d_idx);
            fs::write(file_path, data[d_idx].as_bytes());
        }

        for c_idx in 0..C {
            let file_path = self.checksum_file(self.data_slices, c_idx);
            fs::write(file_path, checksum[c_idx].as_bytes());
        }

        self.data_slices += 1;

        self.data_slices - 1
    }

    pub fn read_data(&self, data_slice: usize, data_idx: usize) -> [u8; X] {
        let file_path = self.data_file(data_slice, data_idx);
        fs::read(file_path).unwrap().try_into().unwrap()
    }
}