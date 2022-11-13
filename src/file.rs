use std::collections::HashMap;
use std::convert::TryInto;
use std::path::PathBuf;

use crate::raid::RAID;
use crate::galois;

#[derive(Debug, Clone)]
struct FileLocation {
    pub start_slice: usize,
    pub start_data_idx: usize,
    pub length: usize,
}

impl FileLocation {
    pub fn new(start_slice: usize, start_data_idx: usize, length: usize) -> Self {
        Self {
            start_slice,
            start_data_idx,
            length,
        }
    }

    fn increment_data_idx<const D: usize>(&mut self) {
        self.start_data_idx += 1;
        if self.start_data_idx == D {
            self.start_data_idx = 0;
            self.start_slice += 1;
        }
    }
}


pub struct FileHandler<R: RAID<D, C, X>, const D: usize, const C: usize, const X: usize>
    where
        [(); D * X]:,
        [(); X * D]:,
{
    raid: R,
    file_locations: HashMap<String, FileLocation>,
    current_slice: usize,
    current_data_idx: usize,
    zero: Box<[u8; X]>
}

impl<R: RAID<D, C, X>, const D: usize, const C: usize, const X: usize> FileHandler<R, D, C, X>
    where
        [(); D * X]:,
        [(); X * D]:,
{

    pub fn new(path: PathBuf) -> Self {
        Self {
            raid: R::new(path),
            file_locations: HashMap::new(),
            current_slice: 0,
            current_data_idx: 0,
            zero: galois::from_fn_raw(|i| 0u8),
        }
    }
    pub fn destroy_devices(&self, dev_idxs: &[usize]) {
        self.raid.destroy_devices(dev_idxs)
    }

    pub fn shutdown(self) {
        self.raid.shutdown()
    }

    fn increment_data_idx(&mut self) {
        self.current_data_idx += 1;
        if self.current_data_idx == D {
            self.current_data_idx = 0;
            self.current_slice += 1;
        }
    }

    pub fn add_file(&mut self, name: String, content: &[u8]) {
        let file_location =
            FileLocation::new(self.current_slice, self.current_data_idx, content.len());
        self.file_locations.insert(name, file_location);
        let (chunks, raw_remainder) = content.as_chunks::<X>();
        let mut chunks: Vec<_> = chunks.iter().collect();
        let remainder = galois::from_fn_raw(|i| {
            if i < raw_remainder.len() {
                raw_remainder[i]
            } else {
                0u8
            }
        });
        if !raw_remainder.is_empty() {
            chunks.push(&remainder);
        }
        let mut chunk_idx = 0;

        while self.current_data_idx != 0 && chunk_idx < chunks.len() {
            debug_assert_eq!(self.raid.read_data_at(self.current_slice, self.current_data_idx), self.zero);
            self.raid
                .update_data(chunks[chunk_idx], self.current_slice, self.current_data_idx);
            self.increment_data_idx();
            chunk_idx += 1;
        }

        while chunk_idx + D - 1 < chunks.len() {
            let data: [&[u8; X]; D] =
                core::array::from_fn(|i| chunks[chunk_idx + i]);
            assert_eq!(self.raid.add_data(&data), self.current_slice);
            self.current_slice += 1;
            chunk_idx += D;
        }
        if chunk_idx >= chunks.len() {
            return;
        }

        let zero_slice: [&[u8; X]; D] = core::array::from_fn(|_| self.zero.as_ref());
        assert_eq!(self.raid.add_data(&zero_slice), self.current_slice);
        while chunk_idx < chunks.len() {
            self.raid
                .update_data(chunks[chunk_idx], self.current_slice, self.current_data_idx);
            self.increment_data_idx();
            chunk_idx += 1;
        }
    }

    pub fn read_file(&self, name: &str) -> Vec<u8> {
        let mut file_location = self.file_locations.get(name).unwrap().clone();
        let mut read_bytes = 0;
        let mut result = Vec::with_capacity(file_location.length);
        while read_bytes + X - 1 < file_location.length {
            result.extend_from_slice(
                self.raid
                    .read_data_at(file_location.start_slice, file_location.start_data_idx)
                    .as_slice(),
            );
            file_location.increment_data_idx::<D>();
            read_bytes += X;
        }
        let left_bytes = file_location.length - read_bytes;
        assert!(left_bytes < X);

        if left_bytes == 0 {
            return result
        }

        result.extend_from_slice(
            &self
                .raid
                .read_data_at(file_location.start_slice, file_location.start_data_idx)
                [..left_bytes],
        );

        result
    }
}
