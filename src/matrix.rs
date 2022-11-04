use crate::galois::Galois;
use std::fmt::{Debug, Formatter};

pub struct Matrix<const M: usize, const N: usize> {
    data: [[Galois; N]; M],
}

impl<const M: usize, const N: usize> Debug for Matrix<M, N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("[\n")?;
        for row in &self.data {
            Debug::fmt(row, f)?;
            f.write_str("\n")?;
        }
        f.write_str("]")
    }
}

impl<const N: usize> Matrix<N, N> {
    pub fn gaussian_elimination(&mut self, mut vec: [Galois; N]) -> [Galois; N] {
        for m in 0..N {
            // swapp if zero
            if self.data[m][m] == Galois::zero() {
                for m_below in m+1..N {
                    if self.data[m_below][m] != Galois::zero() {
                        self.data.swap(m, m_below);
                        vec.swap(m, m_below);
                        break
                    }
                }
            }

            if self.data[m][m] == Galois::zero() {
                panic!("Singular matrix")
            }

            if self.data[m][m] != Galois::one() {
                let scale = Galois::one() / self.data[m][m];
                for i in 0..N {
                    self.data[m][i] *= scale;
                }
                vec[m] *= scale;
            }

            for m_below in m+1..N {
                if self.data[m_below][m] != Galois::zero() {
                    let scale = self.data[m_below][m];
                    for e in 0..N {
                        self.data[m_below][e] -= scale * self.data[m][e]
                    }
                    vec[m_below] -= scale * vec[m]
                }
            }
        }

        for m in (0..N-1).rev() {
            for c in m+1..N {
                vec[m] -= vec[c] * self.data[m][c]
            }
        }

        vec
    }
}

impl<const M: usize, const N: usize> Matrix<M, N> {
    pub fn vandermonde() -> Self {
        let data = core::array::from_fn(|m| core::array::from_fn(|n| Galois::new((n + 1) as u8).pow(m)));
        Self {
            data,
        }
    }

    pub fn recovery_matrix(&self, mut ds: Vec<usize>, cs: Vec<usize>) -> Matrix<N, N> {
        assert_eq!(ds.len() + cs.len(), N);

        let data = core::array::from_fn(|m| {
            if m < ds.len() {
                core::array::from_fn(|n| {
                    if ds[m] == n {
                        Galois::one()
                    } else {
                        Galois::zero()
                    }
                })
            } else {
                self.data[cs[m-ds.len()]].clone()
            }
        });

        Matrix::<N, N> {
            data
        }
    }

    pub fn mul_vec(&self, vec: &[Galois; N]) -> [Galois; M] {
        let mut result = [Galois::zero(); M];
        for (mut r, row) in result.iter_mut().zip(&self.data) {
            for (m, v) in row.iter().zip(vec) {
                *r += m * v;
            }
        }
        result
    }
}