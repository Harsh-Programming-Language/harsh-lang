//! Harsh's standard library: the runtime the prelude's `m~` and `v~` expand to.
//!
//! Julia-style matrices, as a thin wrapper over nalgebra (the user's ruling,
//! 2026-09-21: "what Julia implemented, for familiarity"). `*` between two
//! matrices is the matrix product, as in Julia; indices are 0-based and
//! ranges are Harsh's own, because every other collection in the language is.
//!
//! Harsh has no types, so a macro cannot tell a number from a matrix. The
//! trait system can: `A * B` and `2.0 * A` pick different implementations at
//! compile time -- the dispatch Julia does at run time.

use nalgebra::{DMatrix, DVector, Scalar};
use std::fmt;
use std::ops::{Add, Mul, Sub};

mod broadcast;
mod refs;
mod slicing;
pub use broadcast::{each1, each2, each3, Join, Dot, Half, Operand, Shape, DOT, KM, KS, KV};
pub use slicing::{Pick, Span, VectorView, VectorViewMut, View, ViewMut, ViewMutPick, ViewPick};

/// A matrix. Its shape is the inner matrix's own: one source of truth.
#[derive(Clone, Debug, PartialEq)]
pub struct Matrix<T: Scalar>(pub DMatrix<T>);

/// A vector: a column, as Julia's `Vector` is.
#[derive(Clone, Debug, PartialEq)]
pub struct Vector<T: Scalar>(pub DVector<T>);

impl<T: Scalar + Copy> Matrix<T> {
    /// From rows, as `m~ [1 2; 3 4]` writes them. Ragged rows are refused,
    /// as Julia refuses them.
    #[track_caller]
    pub fn from_rows(rows: Vec<Vec<T>>) -> Self {
        let r = rows.len();
        let c = rows.first().map_or(0, |row| row.len());
        for (i, row) in rows.iter().enumerate() {
            assert!(row.len() == c, "row {} has {} entries, row 1 has {}", i + 1, row.len(), c);
        }
        let flat: Vec<T> = rows.into_iter().flatten().collect();
        Matrix(DMatrix::from_row_slice(r, c, &flat))
    }
    pub fn nrows(&self) -> usize { self.0.nrows() }
    pub fn ncols(&self) -> usize { self.0.ncols() }
    pub fn size(&self) -> (usize, usize) { self.0.shape() }
    pub fn transpose(&self) -> Self { Matrix(self.0.transpose()) }
}

impl<T: Scalar + Copy> Vector<T> {
    pub fn from_vec(v: Vec<T>) -> Self { Vector(DVector::from_vec(v)) }
    pub fn len(&self) -> usize { self.0.len() }
    pub fn is_empty(&self) -> bool { self.0.len() == 0 }
}

// Matrix * Matrix: the matrix product, as Julia's `*`.
/// Julia's error, raised when the program runs: sizes are values, not types.
#[track_caller]
fn mismatch(msg: String) -> ! {
    panic!("DimensionMismatch: {msg}")
}
#[track_caller]
fn check_product(a: (usize, usize), b: (usize, usize)) {
    if a.1 != b.0 {
        mismatch(format!("matrix A has dimensions {a:?}, matrix B has dimensions {b:?}"));
    }
}
#[track_caller]
fn check_same(a: (usize, usize), b: (usize, usize)) {
    if a != b {
        mismatch(format!("dimensions must match: a has dims {a:?}, b has dims {b:?}"));
    }
}

impl<T: Scalar> Mul<Matrix<T>> for Matrix<T>
where DMatrix<T>: Mul<DMatrix<T>, Output = DMatrix<T>> {
    type Output = Matrix<T>;
    #[track_caller]
    fn mul(self, rhs: Matrix<T>) -> Matrix<T> {
        check_product(self.0.shape(), rhs.0.shape());
        Matrix(self.0 * rhs.0)
    }
}

// Matrix * Vector: a vector.
impl<T: Scalar> Mul<Vector<T>> for Matrix<T>
where DMatrix<T>: Mul<DVector<T>, Output = DVector<T>> {
    type Output = Vector<T>;
    #[track_caller]
    fn mul(self, rhs: Vector<T>) -> Vector<T> {
        check_product(self.0.shape(), (rhs.0.len(), 1));
        Vector(self.0 * rhs.0)
    }
}

// Matrix * scalar, scalar * Matrix. The left-scalar form must name each
// number type: Rust's orphan rule forbids a generic `impl Mul<Matrix<T>> for T`.
macro_rules! scalars {
    ($($t:ty)*) => {$(
        impl Mul<$t> for Matrix<$t> {
            type Output = Matrix<$t>;
            #[track_caller]
            fn mul(self, k: $t) -> Matrix<$t> { Matrix(self.0 * k) }
        }
        impl Mul<Matrix<$t>> for $t {
            type Output = Matrix<$t>;
            #[track_caller]
            fn mul(self, m: Matrix<$t>) -> Matrix<$t> { Matrix(m.0 * self) }
        }
        impl Mul<Vector<$t>> for $t {
            type Output = Vector<$t>;
            #[track_caller]
            fn mul(self, v: Vector<$t>) -> Vector<$t> { Vector(v.0 * self) }
        }
    )*};
}
scalars!(f64 f32 i64 i32);

impl<T: Scalar> Add for Matrix<T> where DMatrix<T>: Add<Output = DMatrix<T>> {
    type Output = Matrix<T>;
    #[track_caller]
    fn add(self, rhs: Matrix<T>) -> Matrix<T> {
        check_same(self.0.shape(), rhs.0.shape());
        Matrix(self.0 + rhs.0)
    }
}
impl<T: Scalar> Sub for Matrix<T> where DMatrix<T>: Sub<Output = DMatrix<T>> {
    type Output = Matrix<T>;
    #[track_caller]
    fn sub(self, rhs: Matrix<T>) -> Matrix<T> {
        check_same(self.0.shape(), rhs.0.shape());
        Matrix(self.0 - rhs.0)
    }
}
impl<T: Scalar> Add for Vector<T> where DVector<T>: Add<Output = DVector<T>> {
    type Output = Vector<T>;
    #[track_caller]
    fn add(self, rhs: Vector<T>) -> Vector<T> { Vector(self.0 + rhs.0) }
}

/// `i32`, `f64`, `&str`: the element type as a reader writes it.
fn short_type<T>() -> String {
    std::any::type_name::<T>().rsplit("::").next().unwrap_or("?").to_string()
}

/// Printed as Julia prints: the shape and the element type, then one row
/// per line.
impl<T: Scalar + fmt::Display> fmt::Display for Matrix<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (r, c) = self.0.shape();
        write!(f, "{r}×{c} Matrix<{}>:", short_type::<T>())?;
        for i in 0..r {
            write!(f, "\n")?;
            for j in 0..c { write!(f, " {}", self.0[(i, j)])?; }
        }
        Ok(())
    }
}
impl<T: Scalar + fmt::Display> fmt::Display for Vector<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-element Vector<{}>:", self.0.len(), short_type::<T>())?;
        for x in self.0.iter() { write!(f, "\n {x}")?; }
        Ok(())
    }
}

// Borrowing forms: `&a * &b` leaves both usable, as Julia's `A * B` does.
impl<'a, T: Scalar> Mul<&'a Matrix<T>> for &'a Matrix<T>
where for<'x> &'x DMatrix<T>: Mul<&'x DMatrix<T>, Output = DMatrix<T>> {
    type Output = Matrix<T>;
    #[track_caller]
    fn mul(self, rhs: &'a Matrix<T>) -> Matrix<T> {
        check_product(self.0.shape(), rhs.0.shape());
        Matrix(&self.0 * &rhs.0)
    }
}
impl<'a, T: Scalar> Mul<&'a Vector<T>> for &'a Matrix<T>
where for<'x> &'x DMatrix<T>: Mul<&'x DVector<T>, Output = DVector<T>> {
    type Output = Vector<T>;
    #[track_caller]
    fn mul(self, rhs: &'a Vector<T>) -> Vector<T> {
        check_product(self.0.shape(), (rhs.0.len(), 1));
        Vector(&self.0 * &rhs.0)
    }
}
impl<'a, T: Scalar> Add<&'a Matrix<T>> for &'a Matrix<T>
where for<'x> &'x DMatrix<T>: Add<&'x DMatrix<T>, Output = DMatrix<T>> {
    type Output = Matrix<T>;
    #[track_caller]
    fn add(self, rhs: &'a Matrix<T>) -> Matrix<T> {
        check_same(self.0.shape(), rhs.0.shape());
        Matrix(&self.0 + &rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(rows: Vec<Vec<f64>>) -> Matrix<f64> { Matrix::from_rows(rows) }

    #[test]
    fn star_is_the_matrix_product_scaling_or_a_vector_by_operand_type() {
        let a = m(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let b = m(vec![vec![0.0, 1.0], vec![1.0, 0.0]]);
        assert_eq!(&a * &b, m(vec![vec![2.0, 1.0], vec![4.0, 3.0]]));
        assert_eq!(2.0 * a.clone(), m(vec![vec![2.0, 4.0], vec![6.0, 8.0]]));
        assert_eq!(&a * &Vector::from_vec(vec![1.0, 1.0]), Vector::from_vec(vec![3.0, 7.0]));
    }

    #[test]
    fn shape_transpose_and_display() {
        let a = m(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
        assert_eq!(a.size(), (2, 3));
        assert_eq!(a.transpose().size(), (3, 2));
        assert_eq!(format!("{}", m(vec![vec![1.0, 2.0]])), "1×2 Matrix<f64>:\n 1 2");
    }

    /// The user's Julia transcript, line for line (2026-09-21).
    #[test]
    fn uniform_scaling_as_in_julia() {
        let a: Matrix<i64> = Matrix::from_rows(vec![vec![1, 2], vec![3, 4]]);
        let u = UniformScaling(2);
        assert_eq!(&a + u, Matrix::from_rows(vec![vec![3, 2], vec![3, 6]]));
        assert_eq!(&a * u, Matrix::from_rows(vec![vec![2, 4], vec![6, 8]]));
        assert_eq!(&a + I, Matrix::from_rows(vec![vec![2, 2], vec![3, 5]]));
        assert_eq!(format!("{}", &a * u), "2×2 Matrix<i64>:\n 2 4\n 6 8");
    }

    #[test]
    #[should_panic(expected = "DimensionMismatch: matrix is not square: dimensions are (2, 3)")]
    fn subtracting_a_scaling_needs_a_square_matrix() {
        let b: Matrix<i64> = Matrix::from_rows(vec![vec![1, 2, 3], vec![4, 5, 6]]);
        let _ = &b - UniformScaling(2);
    }

    #[test]
    #[should_panic(expected = "DimensionMismatch: matrix A has dimensions (2, 3), matrix B has dimensions (2, 3)")]
    fn a_product_needs_matching_inner_dimensions() {
        let b: Matrix<i64> = Matrix::from_rows(vec![vec![1, 2, 3], vec![4, 5, 6]]);
        let _ = &b * &b;
    }

    #[test]
    fn a_vector_of_strings() {
        let v: Vector<&str> = Vector::from_vec(vec!["a", "b", "c", "d"]);
        assert_eq!(v.len(), 4);
        assert_eq!(format!("{v}"), "4-element Vector<&str>:\n a\n b\n c\n d");
    }

    #[test]
    #[should_panic(expected = "row 2 has 1 entries, row 1 has 2")]
    fn ragged_rows_are_refused_as_julia_refuses_them() {
        m(vec![vec![1.0, 2.0], vec![3.0]]);
    }
}

/// Julia's `UniformScaling`: `k` times an identity of whatever size the
/// other operand has. `I` is the identity itself. Adding or subtracting
/// needs a square matrix, as in Julia; scaling does not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UniformScaling<T>(pub T);

/// The identity, `UniformScaling(1)` of any number type: `a + I`, `a * I`.
#[allow(non_upper_case_globals)]
pub const I: Identity = Identity;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Identity;

#[track_caller]

fn check_square(a: (usize, usize)) {
    if a.0 != a.1 {
        mismatch(format!("matrix is not square: dimensions are {a:?}"));
    }
}

impl<T: Scalar + Copy + Add<Output = T>> Matrix<T> {
    #[track_caller]
    fn shift_diagonal(&self, k: T) -> Matrix<T> {
        check_square(self.0.shape());
        let mut m = self.0.clone();
        for i in 0..m.nrows() {
            m[(i, i)] = m[(i, i)] + k;
        }
        Matrix(m)
    }
}

impl<T: Scalar + Copy + Add<Output = T>> Add<UniformScaling<T>> for &Matrix<T> {
    type Output = Matrix<T>;
    #[track_caller]
    fn add(self, u: UniformScaling<T>) -> Matrix<T> { self.shift_diagonal(u.0) }
}
impl<T: Scalar + Copy + Add<Output = T>> Add<UniformScaling<T>> for Matrix<T> {
    type Output = Matrix<T>;
    #[track_caller]
    fn add(self, u: UniformScaling<T>) -> Matrix<T> { self.shift_diagonal(u.0) }
}
impl<T: Scalar + Copy + Add<Output = T> + std::ops::Neg<Output = T>> Sub<UniformScaling<T>> for &Matrix<T> {
    type Output = Matrix<T>;
    #[track_caller]
    fn sub(self, u: UniformScaling<T>) -> Matrix<T> { self.shift_diagonal(-u.0) }
}
impl<T: Scalar + Copy + Add<Output = T> + std::ops::Neg<Output = T>> Sub<UniformScaling<T>> for Matrix<T> {
    type Output = Matrix<T>;
    #[track_caller]
    fn sub(self, u: UniformScaling<T>) -> Matrix<T> { self.shift_diagonal(-u.0) }
}
impl<T: Scalar + Copy> Mul<UniformScaling<T>> for &Matrix<T> where DMatrix<T>: Mul<T, Output = DMatrix<T>> {
    type Output = Matrix<T>;
    #[track_caller]
    fn mul(self, u: UniformScaling<T>) -> Matrix<T> { Matrix(self.0.clone() * u.0) }
}
impl<T: Scalar + Copy> Mul<UniformScaling<T>> for Matrix<T> where DMatrix<T>: Mul<T, Output = DMatrix<T>> {
    type Output = Matrix<T>;
    #[track_caller]
    fn mul(self, u: UniformScaling<T>) -> Matrix<T> { Matrix(self.0 * u.0) }
}
impl<T: Scalar + Copy + Add<Output = T> + num_traits::One> Add<Identity> for &Matrix<T> {
    type Output = Matrix<T>;
    #[track_caller]
    fn add(self, _: Identity) -> Matrix<T> { self.shift_diagonal(T::one()) }
}
impl<T: Scalar + Copy> Mul<Identity> for &Matrix<T> {
    type Output = Matrix<T>;
    #[track_caller]
    fn mul(self, _: Identity) -> Matrix<T> { self.clone() }
}

// ---------------------------------------------------------------------------
// Julia's concatenation: the grammar of `m~` (the user's ruling, 2026-09-21).
// A space is `hcat`, `;` or a line break is `vcat`, and every entry is a
// block -- a number a 1×1 block, a matrix itself, a vector its column. So
// `m~ [a b]` joins two matrices side by side exactly as `m~ [1 2]` joins two
// numbers: one rule, applied recursively.
// ---------------------------------------------------------------------------

/// Anything that can stand as a block in a matrix literal.
pub trait IntoBlock<T: Scalar> {
    fn into_block(self) -> Matrix<T>;
}
impl<T: Scalar> IntoBlock<T> for Matrix<T> {
    fn into_block(self) -> Matrix<T> { self }
}
impl<T: Scalar> IntoBlock<T> for &Matrix<T> {
    fn into_block(self) -> Matrix<T> { self.clone() }
}
impl<T: Scalar> IntoBlock<T> for Vector<T> {
    fn into_block(self) -> Matrix<T> {
        let n = self.0.len();
        Matrix(DMatrix::from_iterator(n, 1, self.0.iter().cloned()))
    }
}

/// A number and nothing else: the entries of a comma-separated column.
/// Julia makes `[a, b]` of two matrices a vector *of matrices*, not a matrix,
/// so a matrix is not accepted here.
pub trait Entry<T: Scalar> {
    fn into_entry(self) -> T;
}

macro_rules! numbers {
    ($($t:ty)*) => {$(
        impl IntoBlock<$t> for $t {
            fn into_block(self) -> Matrix<$t> { Matrix(DMatrix::from_element(1, 1, self)) }
        }
        impl Entry<$t> for $t {
            fn into_entry(self) -> $t { self }
        }
    )*};
}
numbers!(f64 f32 i128 i64 i32 i16 i8 u128 u64 u32 u16 u8 isize usize);

/// A block, from anything that can be one.
pub fn block<T: Scalar, B: IntoBlock<T>>(b: B) -> Matrix<T> { b.into_block() }

/// One entry of a column.
pub fn entry<T: Scalar, E: Entry<T>>(e: E) -> Matrix<T> {
    Matrix(DMatrix::from_element(1, 1, e.into_entry()))
}

/// Blocks side by side. They must have the same number of rows.
#[track_caller]
pub fn hcat<T: Scalar>(blocks: Vec<Matrix<T>>) -> Matrix<T> {
    let rows: Vec<usize> = blocks.iter().map(|b| b.0.nrows()).collect();
    if rows.windows(2).any(|w| w[0] != w[1]) {
        mismatch(format!("number of rows of each array must match (got {})", tuple(&rows)));
    }
    let r = rows.first().copied().unwrap_or(0);
    let cols: Vec<T> = blocks.iter().flat_map(|b| b.0.iter().cloned()).collect();
    let c: usize = blocks.iter().map(|b| b.0.ncols()).sum();
    // nalgebra stores column-major, so the blocks' storage, in order, is
    // exactly the joined matrix's.
    Matrix(DMatrix::from_vec(r, c, cols))
}

/// Blocks one above another. They must have the same number of columns.
#[track_caller]
pub fn vcat<T: Scalar>(blocks: Vec<Matrix<T>>) -> Matrix<T> {
    let cols: Vec<usize> = blocks.iter().map(|b| b.0.ncols()).collect();
    if cols.windows(2).any(|w| w[0] != w[1]) {
        mismatch(format!("number of columns of each array must match (got {})", tuple(&cols)));
    }
    let c = cols.first().copied().unwrap_or(0);
    let r: usize = blocks.iter().map(|b| b.0.nrows()).sum();
    Matrix(DMatrix::from_fn(r, c, |i, j| {
        let mut i = i;
        for b in &blocks {
            if i < b.0.nrows() {
                return b.0[(i, j)].clone();
            }
            i -= b.0.nrows();
        }
        unreachable!()
    }))
}

/// `(1, 2)` for Julia's messages; `(1,)` for one.
fn tuple(xs: &[usize]) -> String {
    let parts: Vec<String> = xs.iter().map(|x| x.to_string()).collect();
    if parts.len() == 1 { format!("({},)", parts[0]) } else { format!("({})", parts.join(", ")) }
}

#[cfg(test)]
mod concat_tests {
    use super::*;

    #[test]
    fn hcat_and_vcat_join_blocks_as_julia_does() {
        let col = |a: i64, b: i64| block(Vector::from_vec(vec![a, b]));
        // [[1, 5] [3, 6] [4, 2]] == [1 3 4; 5 6 2]
        let by_columns = hcat(vec![col(1, 5), col(3, 6), col(4, 2)]);
        assert_eq!(by_columns, Matrix::from_rows(vec![vec![1, 3, 4], vec![5, 6, 2]]));
        // [[1 3 4]; [5 6 2]] -- the same matrix, by rows
        let row = |xs: Vec<i64>| hcat(xs.into_iter().map(block).collect());
        assert_eq!(vcat(vec![row(vec![1, 3, 4]), row(vec![5, 6, 2])]), by_columns);
        // [a b] of two matrices: concatenation, for free
        let a: Matrix<i64> = Matrix::from_rows(vec![vec![1, 2], vec![3, 4]]);
        assert_eq!(hcat(vec![block(&a), block(&a)]).size(), (2, 4));
    }

    #[test]
    #[should_panic(expected = "DimensionMismatch: number of rows of each array must match (got (2, 1))")]
    fn hcat_refuses_blocks_of_different_heights() {
        let _ = hcat(vec![block(Vector::from_vec(vec![1i64, 2])), block(3i64)]);
    }
}

// ---------------------------------------------------------------------------
// Solving, inverting, measuring: Julia's `\`, `inv`, `dot`, `norm`
// (2026-09-21). `\` is taken in Harsh, so Julia's `A \ b` is `a <- solve (&b)`
// -- and does what `\` does: an exact solution for a square matrix, the
// least-squares one for a tall matrix, which is linear regression, `β = X \ y`.
// ---------------------------------------------------------------------------

use nalgebra::RealField;


/// Why `try_solve` or `try_inv` gave no answer: the failures `solve` and
/// `inv` panic on, as a value the caller can match (0.1.4). Displayed as
/// Julia's messages, which are also the panics' messages.
#[derive(Debug, Clone, PartialEq)]
pub enum LinAlgError {
    /// The shapes do not fit -- Julia's `DimensionMismatch`.
    DimensionMismatch(String),
    /// The matrix has no inverse -- Julia's `SingularException`.
    Singular,
}

impl std::fmt::Display for LinAlgError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LinAlgError::DimensionMismatch(m) => write!(f, "DimensionMismatch: {m}"),
            LinAlgError::Singular => write!(f, "SingularException: the matrix is singular"),
        }
    }
}

impl std::error::Error for LinAlgError {}

impl<T: RealField + Copy> Matrix<T> {
    /// `A \ b`, the caller deciding what a failure means: square, the exact
    /// solution (LU), `Err(Singular)` when `A` has no inverse; taller or
    /// wider, the least-squares solution (SVD); `Err(DimensionMismatch)`
    /// when `b`'s length is not `A`'s row count. Rust's way, as `RefCell`'s
    /// `try_borrow` is to `borrow` (the user's choice, 2026-09-29).
    pub fn try_solve(&self, b: &Vector<T>) -> Result<Vector<T>, LinAlgError> {
        let (r, c) = self.0.shape();
        if r != b.0.len() {
            return Err(LinAlgError::DimensionMismatch(format!(
                "matrix has {r} rows, right-hand side has length {}",
                b.0.len()
            )));
        }
        if r == c {
            self.0.clone().lu().solve(&b.0).map(Vector).ok_or(LinAlgError::Singular)
        } else {
            let svd = self.0.clone().svd(true, true);
            svd.solve(&b.0, T::default_epsilon()).map(Vector).map_err(|_| LinAlgError::Singular)
        }
    }

    /// Julia's `A \ b`: `try_solve`'s answer, or a panic with Julia's
    /// message. Square: the exact solution (LU). Taller than wide: the
    /// least-squares solution (SVD), as Julia's `\` gives for a tall `A`.
    #[track_caller]
    pub fn solve(&self, b: &Vector<T>) -> Vector<T> {
        match self.try_solve(b) {
            Ok(x) => x,
            Err(e) => panic!("{e}"),
        }
    }

    /// The inverse, the caller deciding what a failure means:
    /// `Err(DimensionMismatch)` when `A` is not square, `Err(Singular)` when
    /// it has no inverse.
    pub fn try_inv(&self) -> Result<Matrix<T>, LinAlgError> {
        let shape = self.0.shape();
        if shape.0 != shape.1 {
            return Err(LinAlgError::DimensionMismatch(format!("matrix is not square: dimensions are {shape:?}")));
        }
        self.0.clone().try_inverse().map(Matrix).ok_or(LinAlgError::Singular)
    }

    /// Julia's `inv(A)`: `try_inv`'s answer, or a panic with Julia's message.
    #[track_caller]
    pub fn inv(&self) -> Matrix<T> {
        match self.try_inv() {
            Ok(m) => m,
            Err(e) => panic!("{e}"),
        }
    }

    /// Julia's `det(A)`.
    #[track_caller]
    pub fn det(&self) -> T {
        check_square(self.0.shape());
        self.0.determinant()
    }
}

impl<T: RealField + Copy> Vector<T> {
    /// Julia's `dot(a, b)`.
    #[track_caller]
    pub fn dot(&self, other: &Vector<T>) -> T {
        if self.0.len() != other.0.len() {
            mismatch(format!("vectors have lengths {} and {}", self.0.len(), other.0.len()));
        }
        self.0.dot(&other.0)
    }
    /// Julia's `norm(v)`: the Euclidean length.
    pub fn norm(&self) -> T { self.0.norm() }
}

impl<T: Scalar> Sub for Vector<T> where DVector<T>: Sub<Output = DVector<T>> {
    type Output = Vector<T>;
    #[track_caller]
    fn sub(self, rhs: Vector<T>) -> Vector<T> { Vector(self.0 - rhs.0) }
}
impl<'a, T: Scalar> Sub<&'a Vector<T>> for &'a Vector<T>
where for<'x> &'x DVector<T>: Sub<&'x DVector<T>, Output = DVector<T>> {
    type Output = Vector<T>;
    #[track_caller]
    fn sub(self, rhs: &'a Vector<T>) -> Vector<T> { Vector(&self.0 - &rhs.0) }
}

/// `v[i]`, 0-based like every collection in Harsh.
impl<T: Scalar> std::ops::Index<usize> for Vector<T> {
    type Output = T;
    #[track_caller]
    fn index(&self, i: usize) -> &T { &self.0[i] }
}
/// `a[(i, j)]`, 0-based.
impl<T: Scalar> std::ops::Index<(usize, usize)> for Matrix<T> {
    type Output = T;
    #[track_caller]
    fn index(&self, ij: (usize, usize)) -> &T { &self.0[ij] }
}

#[cfg(test)]
mod algebra_tests {
    use super::*;
    fn close(a: f64, b: f64) -> bool { (a - b).abs() < 1e-9 }

    #[test]
    fn solve_is_julias_backslash() {
        // Square: 2x + y = 5, x + 3y = 10  =>  x = 1, y = 3.
        let a = Matrix::from_rows(vec![vec![2.0, 1.0], vec![1.0, 3.0]]);
        let x = a.solve(&Vector::from_vec(vec![5.0, 10.0]));
        assert!(close(x[0], 1.0) && close(x[1], 3.0), "{x}");
        // Tall: least squares, a line through (0,1), (1,3), (2,5): y = 1 + 2x.
        let design = Matrix::from_rows(vec![vec![1.0, 0.0], vec![1.0, 1.0], vec![1.0, 2.0]]);
        let beta = design.solve(&Vector::from_vec(vec![1.0, 3.0, 5.0]));
        assert!(close(beta[0], 1.0) && close(beta[1], 2.0), "{beta}");
    }

    #[test]
    fn inverse_determinant_dot_and_norm() {
        let a = Matrix::from_rows(vec![vec![4.0, 7.0], vec![2.0, 6.0]]);
        assert!(close(a.det(), 10.0));
        let id = &a * &a.inv();
        assert!(close(id[(0, 0)], 1.0) && close(id[(0, 1)], 0.0) && close(id[(1, 1)], 1.0));
        let v = Vector::from_vec(vec![3.0, 4.0]);
        assert!(close(v.norm(), 5.0) && close(v.dot(&v), 25.0));
    }

    #[test]
    fn try_solve_and_try_inv_give_the_failure_as_a_value() {
        let a = Matrix::from_rows(vec![vec![2.0, 1.0], vec![1.0, 3.0]]);
        let x = a.try_solve(&Vector::from_vec(vec![5.0, 10.0])).unwrap();
        assert!(close(x[0], 1.0) && close(x[1], 3.0), "{x}");
        let singular = Matrix::from_rows(vec![vec![1.0, 2.0], vec![2.0, 4.0]]);
        assert_eq!(singular.try_solve(&Vector::from_vec(vec![1.0, 2.0])), Err(LinAlgError::Singular));
        assert_eq!(singular.try_inv(), Err(LinAlgError::Singular));
        let e = a.try_solve(&Vector::from_vec(vec![1.0, 2.0, 3.0])).unwrap_err();
        assert_eq!(e.to_string(), "DimensionMismatch: matrix has 2 rows, right-hand side has length 3");
        let wide = Matrix::from_rows(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
        assert_eq!(wide.try_inv().unwrap_err().to_string(), "DimensionMismatch: matrix is not square: dimensions are (2, 3)");
        // A tall matrix, even a degenerate one, has a least-squares answer.
        let tall = Matrix::from_rows(vec![vec![1.0, 1.0], vec![1.0, 1.0], vec![1.0, 1.0]]);
        assert!(tall.try_solve(&Vector::from_vec(vec![1.0, 2.0, 3.0])).is_ok());
    }

    #[test]
    #[should_panic(expected = "DimensionMismatch: matrix has 2 rows, right-hand side has length 3")]
    fn solve_still_panics_with_julias_message() {
        Matrix::from_rows(vec![vec![2.0, 1.0], vec![1.0, 3.0]]).solve(&Vector::from_vec(vec![1.0, 2.0, 3.0]));
    }

    #[test]
    #[should_panic(expected = "SingularException")]
    fn a_singular_matrix_is_refused_as_julia_refuses_it() {
        Matrix::from_rows(vec![vec![1.0, 2.0], vec![2.0, 4.0]]).inv();
    }
}
