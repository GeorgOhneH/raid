#![feature(generic_const_exprs)]

use crate::matrix::Matrix;
use crate::galois::Galois;
use crate::head_node::HeadNode;
use std::path::PathBuf;

pub mod galois;
pub mod matrix;
pub mod head_node;
pub mod matrix_slice;

const D: usize = 3;
const C: usize = 2;


fn main() {
    let path = PathBuf::from("C:\\scripts\\rust\\raid\\disks");
    let _ = std::fs::remove_dir_all(&path);
    let data = [[0u8, 1], [2, 3], [4, 5]];
    let galois_slice = unsafe {core::mem::transmute(data) };
    let mut head_node = HeadNode::<3, 2, 2>::new(path);
    let data_slice = head_node.add_data(&galois_slice);

    println!("{:?}", head_node.read_data(data_slice, 0));
    println!("{:?}", head_node.read_data(data_slice, 1));
    println!("{:?}", head_node.read_data(data_slice, 2));


}
