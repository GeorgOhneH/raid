use crate::galois::{Galois, GaloisSlice};
use std::fmt::{Debug, Formatter};

pub struct MatrixSlice<const M: usize, const N: usize , const X: usize> {
    data: [[GaloisSlice::<X>; N]; M],
}

impl<const M: usize, const N: usize, const X: usize> Debug for MatrixSlice<M, N, X> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("[\n")?;
        for row in &self.data {
            Debug::fmt(row, f)?;
            f.write_str("\n")?;
        }
        f.write_str("]")
    }
}

impl<const N: usize, const X: usize> MatrixSlice<N, N, X> {
    pub fn gaussian_elimination(&mut self, mut vec: [GaloisSlice::<X>; N]) -> [GaloisSlice::<X>; N] {
        for m in 0..N {
            // swapp if zero
            if self.data[m][m] == GaloisSlice::<X>::zero() {
                for m_below in m+1..N {
                    if self.data[m_below][m] != GaloisSlice::<X>::zero() {
                        self.data.swap(m, m_below);
                        vec.swap(m, m_below);
                        break
                    }
                }
            }

            if self.data[m][m] == GaloisSlice::<X>::zero() {
                panic!("Singular matrix")
            }

            if self.data[m][m] != GaloisSlice::<X>::one() {
                let scale = &GaloisSlice::<X>::one() / &self.data[m][m];
                for i in 0..N {
                    self.data[m][i] *= &scale;
                }
                vec[m] *= &scale;
            }

            for m_below in m+1..N {
                if self.data[m_below][m] != GaloisSlice::<X>::zero() {
                    let scale = self.data[m_below][m];
                    for e in 0..N {
                        self.data[m_below][e] -= &(&scale * &self.data[m][e])
                    }
                    vec[m_below] -= &(&scale * &vec[m])
                }
            }
        }

        for m in (0..N-1).rev() {
            for c in m+1..N {
                vec[m] -= &(&vec[c] * &self.data[m][c])
            }
        }

        vec
    }
}

impl<const M: usize, const N: usize, const X: usize> MatrixSlice<M, N, X> {
    pub fn vandermonde() -> Self {
        let data = core::array::from_fn(|m| core::array::from_fn(|n| GaloisSlice::<X>::new((n + 1) as u8).pow(m)));
        Self {
            data,
        }
    }

    pub fn recovery_matrix(&self, mut ds: Vec<usize>, cs: Vec<usize>) -> MatrixSlice<N, N, X> {
        assert_eq!(ds.len() + cs.len(), N);

        let data = core::array::from_fn(|m| {
            if m < ds.len() {
                core::array::from_fn(|n| {
                    if ds[m] == n {
                        GaloisSlice::<X>::one()
                    } else {
                        GaloisSlice::<X>::zero()
                    }
                })
            } else {
                self.data[cs[m-ds.len()]].clone()
            }
        });

        MatrixSlice::<N, N, X> {
            data
        }
    }

    pub fn mul_vec(&self, vec: &[GaloisSlice::<X>; N]) -> [GaloisSlice::<X>; M] {
        let mut result = [GaloisSlice::<X>::zero(); M];
        for (mut r, row) in result.iter_mut().zip(&self.data) {
            for (m, v) in row.iter().zip(vec) {
                *r += &(m * v);
            }
        }
        result
    }
}