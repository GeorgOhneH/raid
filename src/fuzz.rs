#![feature(generic_const_exprs)]
#![feature(slice_as_chunks)]
#![feature(array_chunks)]
#![feature(new_uninit)]
#![feature(const_mut_refs)]

use std::path::PathBuf;

use rand::{Rng, RngCore};

use raid::file::FileHandler;
use raid::galois;
use raid::raid::distributed::Checkpoint;
use raid::raid::controller::Controller;
use raid::raid::RAID;

fn main() {
    const D: usize = 30; // number of data devices
    const C: usize = 3; // number of checksum devices
    const X: usize = 2usize.pow(20); // chunk size

    println!("Controller");
    fuzz_test::<Controller<D, C, X>, D, C, X>(20);
    println!("Checkpoint");
    fuzz_test::<Checkpoint<D, C, X>, D, C, X>(20);

    println!("Controller");
    fuzz_file_test::<Controller<D, C, X>, D, C, X>(20);
    println!("Checkpoint");
    fuzz_file_test::<Checkpoint<D, C, X>, D, C, X>(20);
}

fn fuzz_file_test<R: RAID<D, C, X>, const D: usize, const C: usize, const X: usize>(
    num_data_slices: usize,
) where
    [(); X * D]:,
    [(); D * X]:,
{
    let mut rng = rand::thread_rng();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("nodes");
    let mut file_handler: FileHandler<R, D, C, X> = FileHandler::new(path);
    let mut all_data = vec![];

    for i in 0..num_data_slices {
        println!("Fuzz File Round {i}");
        // generate file
        let length = rng.gen_range(1..X * 10);
        let mut content = vec![0u8; length];
        rng.fill_bytes(&mut content);
        file_handler.add_file(format!("{i}"), &content);
        all_data.push(content);
        let data_read: Vec<_> = (0..i + 1)
            .map(|i| file_handler.read_file(&format!("{i}")))
            .collect();
        assert_eq!(data_read, all_data);

        // update file
        let data_slice = rng.gen_range(0..all_data.len());
        let content = &mut all_data[data_slice];
        let update_size = rng.gen_range(1..content.len() + 1);
        let mut update_content = vec![0u8; update_size];
        rng.fill_bytes(&mut update_content);
        let offset = if content.len() - update_size != 0 {
            rng.gen_range(0..content.len() - update_size)
        } else {
            0
        };
        content[offset..offset + update_size].copy_from_slice(&update_content);
        file_handler.update_file(&format!("{data_slice}"), &update_content, offset);

        let data_read: Vec<_> = (0..i + 1)
            .map(|i| file_handler.read_file(&format!("{i}")))
            .collect();
        assert_eq!(data_read, all_data);

        // destroy disks
        let number_of_failures: usize = rng.gen_range(0..C);
        let mut failures = vec![];
        while failures.len() < number_of_failures {
            let failure: usize = rng.gen_range(0..C + D);
            if !failures.contains(&failure) {
                failures.push(failure)
            }
        }
        file_handler.destroy_devices(&failures);

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
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("nodes");
    let mut node: R = RAID::new(path);

    // create random data
    let mut data: Vec<_> = (0..num_data_slices)
        .map(|_| {
            let mut data = core::array::from_fn(|_| galois::as_bytes(galois::zeros::<X>()));
            for i in 0..D {
                rng.fill_bytes(data[i].as_mut_slice())
            }
            data
        })
        .collect();

    for i in 0..num_data_slices {
        println!("Fuzz RAID Round {i}");

        // store data
        node.add_data(unsafe { core::mem::transmute(&data[i]) }, i);

        let data_read: Vec<_> = (0..i + 1).map(|i| node.read_data(i)).collect();
        assert_eq!(&data_read, &data[..i + 1]);

        // destroy disks
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

        // update data
        let mut changed_data = galois::as_bytes(galois::zeros::<X>());
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
