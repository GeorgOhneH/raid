#![feature(generic_const_exprs)]
#![feature(slice_as_chunks)]
#![feature(array_chunks)]

use crate::galois::Galois;
use crate::matrix::Matrix;
use crate::single::SingleServer;
use std::path::PathBuf;

use crate::distributed::HeadNode;
use crate::file::FileHandler;
use crate::raid::RAID;
use rand::{rngs::StdRng, Rng, RngCore, SeedableRng};

pub mod distributed;
pub mod file;
pub mod galois;
pub mod matrix;
pub mod raid;
pub mod single;

fn main() {
    let path = PathBuf::from("C:\\scripts\\rust\\raid\\file");
    let mut file_handler = FileHandler::<SingleServer<3, 2, 2>, 3, 2, 2>::new(path);
    let data = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    file_handler.add_file(String::from("hello"), &data);

    let read_data = file_handler.read_file(&String::from("hello"));
    assert_eq!(&data, &read_data);

    fuzz_test::<HeadNode<4, 6, 4>, 4, 6, 4>(100);
    test_distributed();
}

fn test_distributed() {
    let path = PathBuf::from("C:\\scripts\\rust\\raid\\distributed");
    let mut data1 = [[0u8, 1], [2, 3], [4, 5]];
    let mut data2 = [[6u8, 7], [8, 9], [10, 11]];
    let galois_slice1 = unsafe { core::mem::transmute(data1) };
    let galois_slice2 = unsafe { core::mem::transmute(data2) };

    let mut head_node = HeadNode::<3, 2, 2>::new(path);

    let data_slice1 = head_node.add_data(&galois_slice1);
    assert_eq!(data1, head_node.read_data(data_slice1));
    let data_slice2 = head_node.add_data(&galois_slice2);

    assert_eq!(data1, head_node.read_data(data_slice1));
    assert_eq!(data2, head_node.read_data(data_slice2));

    head_node.destroy_devices(&[0, 1]);

    assert_eq!(data1, head_node.read_data(data_slice1));
    assert_eq!(data2, head_node.read_data(data_slice2));

    head_node.destroy_devices(&[2, 3]);

    assert_eq!(data1, head_node.read_data(data_slice1));
    assert_eq!(data2, head_node.read_data(data_slice2));

    head_node.destroy_devices(&[4, 0]);

    assert_eq!(data1, head_node.read_data(data_slice1));
    assert_eq!(data2, head_node.read_data(data_slice2));
}

fn test_single() {
    let path = PathBuf::from("C:\\scripts\\rust\\raid\\disks");
    let mut data1 = [[0u8, 1], [2, 3], [4, 5]];
    let mut data2 = [[6u8, 7], [8, 9], [10, 11]];
    let galois_slice1 = unsafe { core::mem::transmute(data1) };
    let galois_slice2 = unsafe { core::mem::transmute(data2) };
    let mut head_node = SingleServer::<3, 2, 2>::new(path);
    let data_slice1 = head_node.add_data(&galois_slice1);
    let data_slice2 = head_node.add_data(&galois_slice2);

    assert_eq!(data1, head_node.read_data(data_slice1));
    assert_eq!(data2, head_node.read_data(data_slice2));

    head_node.remove_device(0);
    head_node.remove_device(1);

    head_node.construct_missing_devices();

    assert_eq!(data1, head_node.read_data(data_slice1));
    assert_eq!(data2, head_node.read_data(data_slice2));

    head_node.remove_device(2);
    head_node.remove_device(3);

    head_node.construct_missing_devices();
    assert_eq!(data1, head_node.read_data(data_slice1));
    assert_eq!(data2, head_node.read_data(data_slice2));

    head_node.remove_device(4);
    head_node.remove_device(0);

    head_node.construct_missing_devices();
    assert_eq!(data1, head_node.read_data(data_slice1));
    assert_eq!(data2, head_node.read_data(data_slice2));

    data1[0] = [9, 9];
    head_node.update_data(&[9, 9], data_slice1, 0);
    data2[2] = [11, 99];
    head_node.update_data(&[11, 99], data_slice2, 2);

    assert_eq!(data1, head_node.read_data(data_slice1));
    assert_eq!(data2, head_node.read_data(data_slice2));

    head_node.remove_device(0);
    head_node.remove_device(1);

    head_node.construct_missing_devices();

    assert_eq!(data1, head_node.read_data(data_slice1));
    assert_eq!(data2, head_node.read_data(data_slice2));

    head_node.remove_device(2);
    head_node.remove_device(3);

    head_node.construct_missing_devices();
    assert_eq!(data1, head_node.read_data(data_slice1));
    assert_eq!(data2, head_node.read_data(data_slice2));

    head_node.remove_device(4);
    head_node.remove_device(0);

    head_node.construct_missing_devices();
    assert_eq!(data1, head_node.read_data(data_slice1));
    assert_eq!(data2, head_node.read_data(data_slice2));

    /*
    let d = [3f64, 5., 8., 10., 15.];
    let matrix = Matrix::<3, 5>::vandermonde();
    let c = matrix.mul_vec(&d);
    let mut rec_matrix = matrix.recovery_matrix(vec![0, 1], vec![0, 1,2 ]);
    println!("{:?}", rec_matrix);
    println!("{:?}", matrix);
    println!("{:?}", c);

    let rec_v = [d[0], d[1], c[0], c[1], c[2]];
    let r = rec_matrix.gaussian_elimination(rec_v);
    println!("{:?}", r);
     */
}

fn fuzz_test<R: RAID<D, C, X>, const D: usize, const C: usize, const X: usize>(
    num_data_slices: usize,
) {
    let mut rng = rand::thread_rng();
    let mut node: R = RAID::new(PathBuf::from("C:\\scripts\\rust\\raid\\fuzz"));

    let mut data: Vec<_> = (0..num_data_slices)
        .map(|_| {
            let mut data = [[0u8; X]; D];
            for i in 0..D {
                rng.fill_bytes(&mut data[i])
            }
            data
        })
        .collect();

    for i in 0..num_data_slices {
        node.add_data(unsafe { core::mem::transmute(&data[i]) });

        let data_read: Vec<_> = (0..i + 1).map(|i| node.read_data(i)).collect();
        assert_eq!(data_read, data[..i + 1]);

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
        assert_eq!(data_read, data[..i + 1]);

        let mut changed_data = [0u8; X];
        rng.fill_bytes(&mut changed_data);
        let data_slice = rng.gen_range(0..i + 1);
        let data_idx = rng.gen_range(0..D);
        data[data_slice][data_idx] = changed_data;
        node.update_data(&changed_data, data_slice, data_idx);

        let data_read: Vec<_> = (0..i + 1).map(|i| node.read_data(i)).collect();
        assert_eq!(data_read, data[..i + 1]);
    }
}
