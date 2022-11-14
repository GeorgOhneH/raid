#![feature(generic_const_exprs)]
#![feature(slice_as_chunks)]
#![feature(array_chunks)]
#![feature(new_uninit)]
#![feature(const_mut_refs)]

use std::path::PathBuf;

use rand::{Rng, RngCore};

use raid::distributed::HeadNode;
use raid::file::FileHandler;
use raid::raid::RAID;
use raid::galois;
use raid::single::SingleServer;

// echo -1 | sudo tee /proc/sys/kernel/perf_event_paranoid
fn main() {
    const X: usize = 2usize.pow(22); // 4MB

    fuzz_file_test::<HeadNode<3, 4, X>, 3, 4, X>(10);
}


fn fuzz_file_test<R: RAID<D, C, X>, const D: usize, const C: usize, const X: usize>(
    num_data_slices: usize,
) where
    [();  X * D ]:,
    [();  D * X ]:,
{
    let mut rng = rand::thread_rng();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("nodes");
    println!("Create FileHandler");
    let mut file_handler: FileHandler<R, D, C, X> =
        FileHandler::new(path);
    let mut all_data = vec![];

    for i in 0..num_data_slices {
        println!("Fuzz File Round {i}");
        let length = rng.gen_range(1..X * D * 3);
        let mut content = vec![0u8; length];
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
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("nodes");
    let mut node: R = RAID::new(path);

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
        node.add_data(unsafe { core::mem::transmute(&data[i]) }, i);

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
