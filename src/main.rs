use crate::matrix::Matrix;
use crate::galois::Galois;

pub mod galois;
pub mod matrix;

fn main() {
    println!("Hello, world!");

    let mut a = Galois::new(10);
    let b = Galois::new(12);
    let c = Galois::new(11);
    let mut d = &mut a;
    d -= Galois::new(8);
    dbg!(d);
    dbg!( Galois::new(10) - Galois::new(8));
    dbg!(10 ^ 8);


    let d = [Galois::new(3), Galois::new(5), Galois::new(8), Galois::new(10), Galois::new(15)];
    let matrix = Matrix::<3, 5>::vandermonde();
    let c = matrix.mul_vec(&d);
    let mut rec_matrix = matrix.recovery_matrix(vec![1, 0, 3, 4, 2], vec![]);
    println!("{:?}", rec_matrix);
    println!("{:?}", matrix);
    println!("{:?}", c);

    let r = rec_matrix.gaussian_elimination(d);
    println!("{:?}", r);


}
