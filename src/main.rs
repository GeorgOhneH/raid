#![feature(generic_const_exprs)]

use crate::matrix::Matrix;
use crate::galois::Galois;
use crate::head_node::HeadNode;
use std::path::PathBuf;

use rand::{rngs::StdRng, RngCore, SeedableRng, Rng};

pub mod galois;
pub mod matrix;
pub mod head_node;


fn main() {
    fuzz_test();
    let path = PathBuf::from("C:\\scripts\\rust\\raid\\disks");
    let mut data1 = [[0u8, 1], [2, 3], [4, 5]];
    let mut data2 = [[6u8, 7], [8, 9], [10, 11]];
    let galois_slice1 = unsafe { core::mem::transmute(data1) };
    let galois_slice2 = unsafe { core::mem::transmute(data2) };
    let mut head_node = HeadNode::<3, 2, 2>::new(path);
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
    head_node.update_data([Galois::new(9), Galois::new(9)], data_slice1, 0);
    data2[2] = [11, 99];
    head_node.update_data([Galois::new(11), Galois::new(99)], data_slice2, 2);

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

fn fuzz_test() {
    /*
    1,1,1,1,1,1
    1,2,3,4,5,6
    1,8,15,64,85,120

    1,1,1,1,1,1
    0,3,2,5,4,7
    0,0,0,90,72,108
    No independent :(
     */
    const D: usize = 9;
    const C: usize = 3;
    const X: usize = 1024;
    const T: usize = 100;
    let mut rng = rand::thread_rng();
    let mut node = HeadNode::<D, C, X>::new(PathBuf::from("C:\\scripts\\rust\\raid\\fuzz"));

    let data: Vec<_> = (0..T).map(|_| {
        let mut data = [[0u8;X]; D];
        for i in 0..D {
            rng.fill_bytes(&mut data[i])
        }
        data
    }).collect();

    for d in &data {
        node.add_data(unsafe { core::mem::transmute(d) });
    }

    let data_read: Vec<_> = (0..T).map(|i| {
        node.read_data(i)
    }).collect();

    assert_eq!(data_read, data);

    for _ in 0..100 {
        let number_of_failures: usize = rng.gen_range(0..C);
        let mut failures = vec![];
        while failures.len() < number_of_failures {
            let failure: usize = rng.gen_range(0..C + D);
            if !failures.contains(&failure) {
                failures.push(failure)
            }
        }
        for failure in failures {
            node.remove_device(failure)
        }

        node.construct_missing_devices();

        let data_read: Vec<_> = (0..T).map(|i| {
            node.read_data(i)
        }).collect();
        assert_eq!(data_read, data);
    }


}

