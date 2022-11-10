use std::fs;
use std::fs::create_dir;
use std::path::{PathBuf};
use std::prelude::rust_2021::TryInto;

use crate::galois;
use crate::galois::Galois;
use crate::matrix::Matrix;
use crate::raid::RAID;

pub struct SingleServer<const D: usize, const C: usize, const X: usize>
where
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    data_slices: usize,
    vandermonde: Matrix<C, D>,
    paths: [PathBuf; C + D],
}

impl<const D: usize, const C: usize, const X: usize> SingleServer<D, C, X>
where
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    fn folder_id(data_slice: usize, data_idx: usize) -> usize {
        (data_idx + data_slice) % (D + C)
    }

    fn data_name(data_slice: usize, data_idx: usize) -> String {
        format!("{}_{}d.bin", data_slice, data_idx)
    }

    fn checksum_name(data_slice: usize, check_idx: usize) -> String {
        format!("{}_{}c.bin", data_slice, check_idx)
    }

    fn data_file(&self, data_slice: usize, data_idx: usize) -> PathBuf {
        let folder_path = &self.paths[Self::folder_id(data_slice, data_idx)];
        let name = Self::data_name(data_slice, data_idx);
        folder_path.join(name)
    }

    fn checksum_file(&self, data_slice: usize, check_idx: usize) -> PathBuf {
        let folder_path = &self.paths[Self::folder_id(data_slice, D + check_idx)];
        let name = Self::checksum_name(data_slice, check_idx);
        folder_path.join(name)
    }

    pub fn read_checksum_at(&self, data_slice: usize, check_idx: usize) -> Box<[u8; X]> {
        let file_path = self.checksum_file(data_slice, check_idx);
        fs::read(file_path).unwrap().try_into().unwrap()
    }

    pub fn read_checksum(&self, data_slice: usize) -> [Box<[u8; X]>; C] {
        core::array::from_fn(|i| self.read_checksum_at(data_slice, i))
    }

    pub fn remove_device(&self, idx: usize) {
        let device_path = &self.paths[idx];
        let _ = std::fs::remove_dir_all(device_path);
    }

    pub fn construct_missing_devices(&self) {
        let mut online_devices: [bool; D + C] = [false; D + C];
        let mut count = 0;
        for i in 0..D + C {
            if self.paths[i].exists() {
                online_devices[i] = true;
                count += 1;
            } else {
                create_dir(&self.paths[i]).unwrap()
            }
        }

        if count < D {
            panic!("Too man devices lost")
        }

        let mut recover_devices = online_devices;

        let mut x = D + C - 1;
        while count > D {
            if recover_devices[x] {
                recover_devices[x] = false;
                count -= 1;
            }
            x -= 1;
        }

        assert_eq!(count, D);

        for data_slice in 0..self.data_slices {
            let mut r_data_check = vec![];
            let mut r_data_idx = vec![];
            for data_idx in 0..D {
                let folder_id_i = Self::folder_id(data_slice, data_idx);
                if recover_devices[folder_id_i] {
                    let bytes = self.read_data_at(data_slice, data_idx);
                    let g_data = galois::from_bytes(bytes);
                    r_data_check.push(g_data);
                    r_data_idx.push(data_idx);
                }
            }
            let mut r_check_idx = vec![];
            for check_idx in 0..C {
                let folder_id_i = Self::folder_id(data_slice, check_idx + D);
                if recover_devices[folder_id_i] {
                    let bytes = self.read_checksum_at(data_slice, check_idx);
                    let g_data = galois::from_bytes(bytes);
                    r_data_check.push(g_data);
                    r_check_idx.push(check_idx);
                }
            }

            let mut rec_matrix = self.vandermonde.recovery_matrix(r_data_idx, r_check_idx);
            let mut data: [Box<[Galois; X]>; D] = r_data_check.try_into().unwrap();
            rec_matrix.gaussian_elimination(&mut data);

            for data_idx in 0..D {
                let folder_id_i = Self::folder_id(data_slice, data_idx);
                if !online_devices[folder_id_i] {
                    let file_path = self.data_file(data_slice, data_idx);
                    fs::write(file_path, galois::as_bytes_ref(&data[data_idx])).unwrap();
                }
            }
            for check_idx in 0..C {
                let folder_id_i = Self::folder_id(data_slice, check_idx + D);
                if !online_devices[folder_id_i] {
                    let file_path = self.checksum_file(data_slice, check_idx);
                    let checksum = self.vandermonde.mul_vec_at(&data, check_idx);
                    fs::write(file_path, galois::as_bytes_ref(&checksum)).unwrap();
                }
            }
        }
    }
}

impl<const D: usize, const C: usize, const X: usize> RAID<D, C, X> for SingleServer<D, C, X>
where
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    fn new(root_path: PathBuf) -> Self {
        let paths = core::array::from_fn(|i| root_path.join(format!("device{i}")));
        for path in &paths {
            let _ = std::fs::remove_dir_all(path);
            create_dir(path).unwrap()
        }

        Self {
            data_slices: 0,
            vandermonde: Matrix::<C, D>::reed_solomon(),
            paths,
        }
    }

    fn add_data(&mut self, data: &[&[u8; X]; D]) -> usize {
        let data: &[&[Galois; X]; D] = unsafe { core::mem::transmute(data) };
        let checksum = self.vandermonde.mul_vec(data);

        for d_idx in 0..D {
            let file_path = self.data_file(self.data_slices, d_idx);
            fs::write(file_path, galois::as_bytes_ref(&data[d_idx])).unwrap();
        }

        for c_idx in 0..C {
            let file_path = self.checksum_file(self.data_slices, c_idx);
            fs::write(file_path, galois::as_bytes_ref(&checksum[c_idx])).unwrap();
        }

        self.data_slices += 1;

        self.data_slices - 1
    }

    fn read_data_at(&self, data_slice: usize, data_idx: usize) -> Box<[u8; X]> {
        let file_path = self.data_file(data_slice, data_idx);
        fs::read(file_path).unwrap().try_into().unwrap()
    }

    fn read_data(&self, data_slice: usize) -> [Box<[u8; X]>; D] {
        core::array::from_fn(|i| self.read_data_at(data_slice, i))
    }

    fn destroy_devices(&self, dev_idxs: &[usize]) {
        for dev_idx in dev_idxs {
            let device_path = &self.paths[*dev_idx];
            let _ = std::fs::remove_dir_all(device_path);
        }
        self.construct_missing_devices()
    }

    fn update_data(&self, data: &[u8; X], data_slice: usize, data_idx: usize) {
        let data = galois::from_bytes_ref(data);
        let old_data = galois::from_bytes(self.read_data_at(data_slice, data_idx));
        let dfile_path = self.data_file(data_slice, data_idx);
        fs::remove_file(&dfile_path).unwrap();
        fs::write(&dfile_path, galois::as_bytes_ref(&data)).unwrap();

        for check_idx in 0..C {
            let old_checksum = galois::from_bytes(self.read_checksum_at(data_slice, check_idx));
            let new_checksum: [Galois; X] = core::array::from_fn(|i| {
                old_checksum[i] + self.vandermonde[check_idx][data_idx] * (data[i] - old_data[i])
            });
            let file_path = self.checksum_file(data_slice, check_idx);
            fs::remove_file(&file_path).unwrap();
            fs::write(&file_path, galois::as_bytes_ref(&new_checksum)).unwrap();
        }
    }
}
