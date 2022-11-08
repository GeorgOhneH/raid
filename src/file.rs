use crate::raid::RAID;
use std::collections::HashMap;
use std::convert::TryInto;
use std::path::PathBuf;

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
    [(); { D * X }]:,
    [(); { X * D }]:,
{
    raid: R,
    file_locations: HashMap<String, FileLocation>,
    current_slice: usize,
    current_data_idx: usize,
}

impl<R: RAID<D, C, X>, const D: usize, const C: usize, const X: usize> FileHandler<R, D, C, X>
where
    [(); { D * X }]:,
    [(); { X * D }]:,
{
    pub fn new(path: PathBuf) -> Self {
        Self {
            raid: R::new(path),
            file_locations: HashMap::new(),
            current_slice: 0,
            current_data_idx: 0,
        }
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
        if self.current_data_idx != 0 && content.len() <= (D - self.current_data_idx) * X {
            let (chunks, remainder) = content.as_chunks::<X>();
            for chunk in chunks {
                self.raid
                    .update_data(chunk, self.current_slice, self.current_data_idx);
                self.increment_data_idx()
            }

            if remainder.len() != 0 {
                let data = core::array::from_fn(|i| {
                    if i < remainder.len() {
                        remainder[i]
                    } else {
                        0u8
                    }
                });
                self.raid
                    .update_data(&data, self.current_slice, self.current_data_idx);
                self.increment_data_idx()
            }
            return;
        }

        let rest = if self.current_data_idx != 0 {
            let (begining, rest) = content.split_at((D - self.current_data_idx) * X);
            for chunk in begining.array_chunks::<X>() {
                self.raid
                    .update_data(chunk, self.current_slice, self.current_data_idx);
                self.increment_data_idx()
            }
            rest
        } else {
            content
        };

        let (chunks, remainder) = rest.as_chunks::<{ X * D }>();
        for chunk in chunks {
            let (data, _) = chunk.as_chunks::<X>();
            self.raid.add_data(data.try_into().unwrap());
            self.current_slice += 1
        }

        let (chunks, remainder) = remainder.as_chunks::<X>();
        let mut last_data = chunks.to_vec();
        if remainder.len() != 0 {
            let data = core::array::from_fn(|i| {
                if i < remainder.len() {
                    remainder[i]
                } else {
                    0u8
                }
            });
            last_data.push(data);
        }
        for _ in 0..last_data.len() {
            self.increment_data_idx()
        }
        while last_data.len() < D {
            last_data.push([0u8; X])
        }
        self.raid.add_data(&last_data.try_into().unwrap());
        self.current_slice += 1
    }

    pub fn read_file(&self, name: &String) -> Vec<u8> {
        let mut file_location = self.file_locations.get(name).unwrap().clone();
        let mut read_bytes = 0;
        let mut result = Vec::with_capacity(file_location.length);
        while read_bytes + X < file_location.length {
            result.extend_from_slice(
                &self
                    .raid
                    .read_data_at(file_location.start_slice, file_location.start_data_idx),
            );
            file_location.increment_data_idx::<D>();
            read_bytes += X;
        }
        let left_bytes = file_location.length - read_bytes;
        assert!(left_bytes <= X);

        result.extend_from_slice(
            &self
                .raid
                .read_data_at(file_location.start_slice, file_location.start_data_idx)
                [..left_bytes],
        );

        result
    }
}
