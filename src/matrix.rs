use std::fmt::{Debug, Formatter};
use std::ops::{Index, IndexMut};

use crate::galois;
use crate::galois::Galois;

// http://web.eecs.utk.edu/~jplank/plank/papers/CS-96-332.pdf
// http://web.eecs.utk.edu/~jplank/plank/papers/CS-03-504.pdf
/// Simply implementation of linear algebra things  
#[derive(Clone)]
pub struct Matrix<const M: usize, const N: usize>
where
    [(); M + N]:,
    [(); N + N]:,
{
    data: [[Galois; N]; M],
}

impl<const M: usize, const N: usize> Debug for Matrix<M, N>
where
    [(); M + N]:,
    [(); N + N]:,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("[\n")?;
        for row in &self.data {
            Debug::fmt(row, f)?;
            f.write_str("\n")?;
        }
        f.write_str("]")
    }
}

impl<const M: usize, const N: usize> Index<usize> for Matrix<M, N>
where
    [(); M + N]:,
    [(); N + N]:,
{
    type Output = [Galois; N];

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<const M: usize, const N: usize> IndexMut<usize> for Matrix<M, N>
where
    [(); M + N]:,
    [(); N + N]:,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

impl<const N: usize> Matrix<N, N>
where
    [(); N + N]:,
{
    pub fn gaussian_elimination<const X: usize>(&mut self, vec: &mut [Box<[Galois; X]>; N]) {
        for m in 0..N {
            // swapp if zero
            if self.data[m][m] == Galois::zero() {
                for m_below in m + 1..N {
                    if self.data[m_below][m] != Galois::zero() {
                        self.data.swap(m, m_below);
                        vec.swap(m, m_below);
                        break;
                    }
                }
            }

            if self.data[m][m] == Galois::zero() {
                println!("{:?}", self);
                panic!("Singular matrix")
            }

            // scale row
            if self.data[m][m] != Galois::one() {
                let scale = Galois::one() / self.data[m][m];
                for i in 0..N {
                    self.data[m][i] *= scale;
                }
                for x_idx in 0..X {
                    vec[m][x_idx] *= scale;
                }
            }

            // subract row to lower one
            for m_below in m + 1..N {
                if self.data[m_below][m] != Galois::zero() {
                    let scale = self.data[m_below][m];
                    for e in 0..N {
                        self.data[m_below][e] -= scale * self.data[m][e]
                    }
                    for x_idx in 0..X {
                        vec[m_below][x_idx] -= scale * vec[m][x_idx]
                    }
                }
            }
        }

        // calculate final output
        for m in (0..N - 1).rev() {
            for c in m + 1..N {
                for x_idx in 0..X {
                    vec[m][x_idx] -= vec[c][x_idx] * self.data[m][c]
                }
            }
        }
    }
}

impl<const M: usize, const N: usize> Matrix<M, N>
where
    [(); M + N]:,
    [(); N + N]:,
{
    /// implementation of http://web.eecs.utk.edu/~jplank/plank/papers/CS-03-504.pdf
    pub fn reed_solomon() -> Self {
        let mut reed: [[Galois; N]; M + N] =
            core::array::from_fn(|m| core::array::from_fn(|n| Galois::new((m) as u8).pow(n)));

        for idx_n in 0..N {
            if reed[idx_n][idx_n] == Galois::zero() {
                for below_n in idx_n + 1..N + M {
                    if reed[below_n][idx_n] != Galois::zero() {
                        reed.swap(below_n, idx_n)
                    }
                }
            }

            if reed[idx_n][idx_n] == Galois::zero() {
                panic!("should never be possible with a vandermonde matrix")
            }

            if reed[idx_n][idx_n] != Galois::one() {
                let scale = Galois::one() / reed[idx_n][idx_n];
                for r in 0..N + M {
                    reed[r][idx_n] *= scale
                }
            }

            for c in (0..idx_n).chain(idx_n + 1..N) {
                let scale = reed[idx_n][c];
                for r in 0..N + M {
                    reed[r][c] -= scale * reed[r][idx_n]
                }
            }
        }

        let data = core::array::from_fn(|i| reed[i + N]);

        Self { data }
    }

    /// construct matrix with know chunk valuess
    pub fn recovery_matrix(&self, ds: Vec<usize>, cs: Vec<usize>) -> Matrix<N, N> {
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
                self.data[cs[m - ds.len()]]
            }
        });

        Matrix::<N, N> { data }
    }

    /// normal matrix vector multiplication
    pub fn mul_vec<const X: usize>(&self, vec: &[&[Galois; X]; N]) -> [Box<[Galois; X]>; M] {
        let mut result = core::array::from_fn(|_| galois::zeros());
        for (r, row) in result.iter_mut().zip(&self.data) {
            for (m, v) in row.iter().zip(vec) {
                for x_idx in 0..X {
                    r[x_idx] += m * v[x_idx];
                }
            }
        }
        result
    }

    /// normal matrix vector multiplication for one index
    pub fn mul_vec_at<const X: usize>(
        &self,
        vec: &[Box<[Galois; X]>; N],
        idx: usize,
    ) -> Box<[Galois; X]> {
        let mut r = galois::zeros();
        let row = &self.data[idx];
        for (m, v) in row.iter().zip(vec) {
            for x_idx in 0..X {
                r[x_idx] += m * v[x_idx];
            }
        }
        r
    }
}
