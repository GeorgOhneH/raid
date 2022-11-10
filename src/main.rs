#![feature(generic_const_exprs)]
#![feature(slice_as_chunks)]
#![feature(array_chunks)]

use std::path::PathBuf;

use rand::{Rng, RngCore};

use crate::distributed::HeadNode;
use crate::file::FileHandler;
use crate::raid::RAID;
use crate::single::SingleServer;

pub mod distributed;
pub mod file;
pub mod galois;
pub mod matrix;
pub mod raid;
pub mod single;

// echo -1 | sudo tee /proc/sys/kernel/perf_event_paranoid
fn main() {
    const X: usize = 4194304 / 16;
    fuzz_file_test::<HeadNode<6, 4, X>, 6, 4, X>(100);
}


fn fuzz_file_test<R: RAID<D, C, X>, const D: usize, const C: usize, const X: usize>(
    num_data_slices: usize,
) where
    [();  X * D ]:,
    [();  D * X ]:,
{
    let mut rng = rand::thread_rng();
    // "C:\\scripts\\rust\\raid\\fuzzfile"
    // "/mnt/c/scripts/rust/raid/fuzzfile"
    let mut file_handler: FileHandler<R, D, C, X> =
        FileHandler::new(PathBuf::from("C:\\scripts\\rust\\raid\\fuzzfile"));

    let mut all_data = vec![];

    for i in 0..num_data_slices {
        println!("Fuzz File Round {i}");
        let length = rng.gen_range(1..X * (D + C) * 10);
        let mut content = Vec::with_capacity(length);
        rng.fill_bytes(&mut content);
        println!("add_file {i}");
        file_handler.add_file(format!("{i}"), &content);
        println!("data_read {i}");
        all_data.push(content);
        println!("data_read {i}");
        let data_read: Vec<_> = (0..i + 1)
            .map(|i| file_handler.read_file(&format!("{i}")))
            .collect();
        assert_eq!(data_read, all_data);

        let number_of_failures: usize = rng.gen_range(0..C);
        let mut failures = vec![];
        while failures.len() < number_of_failures {
            let failure: usize = rng.gen_range(0..C + D);
            if !failures.contains(&failure) {
                failures.push(failure)
            }
        }

        println!("destroy_devices {i}");
        file_handler.destroy_devices(&failures);

        println!("data_read {i}");
        let data_read: Vec<_> = (0..i + 1)
            .map(|i| file_handler.read_file(&format!("{i}")))
            .collect();
        assert_eq!(data_read, all_data);
    }
    file_handler.shutdown()
}

fn fuzz_test<R: RAID<D, C, X>, const D: usize, const C: usize, const X: usize>(
    num_data_slices: usize,
) {
    let mut rng = rand::thread_rng();
    let mut node: R = RAID::new(PathBuf::from("C:\\scripts\\rust\\raid\\fuzz"));

    let mut data: Vec<_> = (0..num_data_slices)
        .map(|_| {
            let mut data = core::array::from_fn(|i| Box::new([0u8; X]));
            for i in 0..D {
                rng.fill_bytes(data[i].as_mut_slice())
            }
            data
        })
        .collect();

    for i in 0..num_data_slices {
        node.add_data(unsafe { core::mem::transmute(&data[i]) });

        let data_read: Vec<_> = (0..i + 1).map(|i| node.read_data(i)).collect();
        assert_eq!(&data_read, &data[..i + 1]);

        let number_of_failures: usize = rng.gen_range(0..C);
        let mut failures = vec![];
        while failures.len() < number_of_failures {
            let failure: usize = rng.gen_range(0..C + D);
            if !failures.contains(&failure) {
                failures.push(failure)
            }
        }

        node.destroy_devices(&failures);

        let data_read: Vec<_> = (0..i + 1).map(|i| node.read_data(i)).collect();
        assert_eq!(&data_read, &data[..i + 1]);

        let mut changed_data = Box::new([0u8; X]);
        rng.fill_bytes(changed_data.as_mut_slice());
        let data_slice = rng.gen_range(0..i + 1);
        let data_idx = rng.gen_range(0..D);
        node.update_data(&changed_data, data_slice, data_idx);
        data[data_slice][data_idx] = changed_data;

        let data_read: Vec<_> = (0..i + 1).map(|i| node.read_data(i)).collect();
        assert_eq!(&data_read, &data[..i + 1]);
    }
    node.shutdown()
}
