//! Slicing: Julia's `a[1:2, :]`, in Harsh's own ranges and 0-based.
//!
//! `..` alone is Julia's `:`. Two spellings, one meaning (ruled 2026-09-21):
//!
//! - **the method**, `a <- slice (0..2) (..)` -- a copy, as Julia's is;
//! - **the index**, `&a[0..2, ..]` -- a borrowed *view*, as Rust's own
//!   `&v[1..3]` is, with `<- copy$` for Julia's copy. (`a[i, j]` is written
//!   with a comma in Harsh; the transpiler makes the tuple Rust wants.)
//!
//! As in Julia, an axis taken by an integer is dropped: `a[1, ..]` is a
//! vector, `a[1, 2]` a number. Harsh has no types; the trait system picks.

use crate::{short_type, Matrix, Vector};
use nalgebra::{DVector, Scalar};
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

/// One axis of a slice: any of Rust's ranges.
pub trait Span {
    /// `(start, length)` within an axis of length `n`, or Julia's `BoundsError`.
    fn span(&self, n: usize) -> (usize, usize);
}
#[track_caller]
fn checked(start: usize, end: usize, n: usize) -> (usize, usize) {
    if start > end || end > n {
        panic!("BoundsError: attempt to access an axis of length {n} at {start}..{end}");
    }
    (start, end - start)
}
impl Span for Range<usize> { #[track_caller] fn span(&self, n: usize) -> (usize, usize) { checked(self.start, self.end, n) } }
impl Span for RangeInclusive<usize> { #[track_caller] fn span(&self, n: usize) -> (usize, usize) { checked(*self.start(), *self.end() + 1, n) } }
impl Span for RangeFrom<usize> { #[track_caller] fn span(&self, n: usize) -> (usize, usize) { checked(self.start, n, n) } }
impl Span for RangeTo<usize> { #[track_caller] fn span(&self, n: usize) -> (usize, usize) { checked(0, self.end, n) } }
impl Span for RangeToInclusive<usize> { #[track_caller] fn span(&self, n: usize) -> (usize, usize) { checked(0, self.end + 1, n) } }
impl Span for RangeFull { fn span(&self, n: usize) -> (usize, usize) { (0, n) } }

#[track_caller]
fn one(i: usize, n: usize) -> usize {
    if i >= n { panic!("BoundsError: attempt to access an axis of length {n} at index {i}"); }
    i
}

/// What a pair of axes picks out of a matrix, by the types of the pair.
pub trait Pick<T: Scalar> {
    type Out;
    fn pick(self, m: &Matrix<T>) -> Self::Out;
}
impl<T: Scalar + Copy, R: Span, C: Span> Pick<T> for (R, C) {
    type Out = Matrix<T>;
    #[track_caller]
    fn pick(self, m: &Matrix<T>) -> Matrix<T> {
        let ((r0, nr), (c0, nc)) = (self.0.span(m.0.nrows()), self.1.span(m.0.ncols()));
        Matrix(m.0.view((r0, c0), (nr, nc)).into_owned())
    }
}
impl<T: Scalar + Copy, C: Span> Pick<T> for (usize, C) {
    type Out = Vector<T>;
    #[track_caller]
    fn pick(self, m: &Matrix<T>) -> Vector<T> {
        let (i, (c0, nc)) = (one(self.0, m.0.nrows()), self.1.span(m.0.ncols()));
        Vector(DVector::from_iterator(nc, (c0..c0 + nc).map(|j| m.0[(i, j)])))
    }
}
impl<T: Scalar + Copy, R: Span> Pick<T> for (R, usize) {
    type Out = Vector<T>;
    #[track_caller]
    fn pick(self, m: &Matrix<T>) -> Vector<T> {
        let ((r0, nr), j) = (self.0.span(m.0.nrows()), one(self.1, m.0.ncols()));
        Vector(DVector::from_iterator(nr, (r0..r0 + nr).map(|i| m.0[(i, j)])))
    }
}
impl<T: Scalar + Copy> Pick<T> for (usize, usize) {
    type Out = T;
    #[track_caller]
    fn pick(self, m: &Matrix<T>) -> T { m.0[(one(self.0, m.0.nrows()), one(self.1, m.0.ncols()))] }
}

impl<T: Scalar + Copy> Matrix<T> {
    /// `a <- slice (0..2) (..)`: a copy of the rows and columns named.
    #[track_caller]
    pub fn slice<R, C>(&self, rows: R, cols: C) -> <(R, C) as Pick<T>>::Out
    where (R, C): Pick<T> {
        (rows, cols).pick(self)
    }
}
impl<T: Scalar + Copy> Vector<T> {
    /// `v <- slice (1..)`: a copy of the entries named.
    #[track_caller]
    pub fn slice<R: Span>(&self, r: R) -> Vector<T> {
        let (s, n) = r.span(self.0.len());
        Vector(DVector::from_iterator(n, (s..s + n).map(|i| self.0[i])))
    }
}

// ---------------------------------------------------------------- views
//
// `Index::index` must return a reference, so it cannot return a copy. A
// reference to an *unsized* type is two words: the address and one spare word,
// which for a slice is its length. `View<T>` wraps `[()]` -- a slice of
// zero-sized entries, which occupies no memory whatever its length -- points
// its address at the matrix it borrows, and keeps the window in the spare
// word. This is the technique `bitvec` uses to make `&bits[1..3]` work.
//
// SAFETY, stated once for every `unsafe` below. The address is always that of
// a live `Matrix<T>` borrowed for the view's whole lifetime (`&self` in, `&View`
// out, same lifetime; likewise `&mut`), so reading the matrix through it is
// reading through the borrow. The `[()]` is never read. A `[()]` of any length
// is valid: its size is `len * 0`.
//
// **Miri's verdict (the user's Mac, 2026-09-25): undefined behaviour under
// Stacked Borrows, accepted by Tree Borrows.** Widening the zero-sized reborrow
// back to the matrix (`parts`, `parts_mut`) is rejected by Stacked Borrows: a
// zero-sized reference carries no permission to the matrix's bytes. Tree
// Borrows accepts every view test. Everything else in this crate is clean
// under Stacked Borrows (`cargo +nightly miri test -- --skip slicing --skip
// broadcast`, 15 tests). Released so in 0.1.30 by the user's decision, stated
// in its changelog; the user chose the remedy, a view as a value with a
// spelling of its own, as the first improvement after it.

/// The window, in one word: two axis codes, each `start * (n + 1) + length`.
#[track_caller]
fn pack(shape: (usize, usize), rows: (usize, usize), cols: (usize, usize)) -> usize {
    let (r, c) = (shape.0 as u128 + 1, shape.1 as u128 + 1);
    if r * r * c * c > usize::MAX as u128 {
        panic!("a view's window does not fit in a reference for a {}×{} matrix on this machine; use `slice`", shape.0, shape.1);
    }
    let rc = rows.0 * (shape.0 + 1) + rows.1;
    let cc = cols.0 * (shape.1 + 1) + cols.1;
    rc * (shape.1 + 1) * (shape.1 + 1) + cc
}
fn unpack(shape: (usize, usize), w: usize) -> [usize; 4] {
    let c2 = (shape.1 + 1) * (shape.1 + 1);
    let (rc, cc) = (w / c2, w % c2);
    [rc / (shape.0 + 1), rc % (shape.0 + 1), cc / (shape.1 + 1), cc % (shape.1 + 1)]
}

macro_rules! view_type {
    ($(#[$doc:meta])* $V:ident) => {
        $(#[$doc])*
        #[repr(transparent)]
        pub struct $V<T: Scalar>(PhantomData<T>, [()]);
        impl<T: Scalar + Copy> $V<T> {
            fn of(m: &Matrix<T>, w: usize) -> &Self {
                // SAFETY: see the note above.
                unsafe { &*(std::slice::from_raw_parts(m as *const Matrix<T> as *const (), w) as *const [()] as *const Self) }
            }
            fn of_mut(m: &mut Matrix<T>, w: usize) -> &mut Self {
                // SAFETY: see the note above.
                unsafe { &mut *(std::slice::from_raw_parts_mut(m as *mut Matrix<T> as *mut (), w) as *mut [()] as *mut Self) }
            }
            /// The matrix looked into, and `[row start, rows, column start, columns]`.
            pub(crate) fn parts(&self) -> (&Matrix<T>, [usize; 4]) {
                // SAFETY: see the note above.
                let m = unsafe { &*(self as *const Self as *const () as *const Matrix<T>) };
                (m, unpack(m.0.shape(), self.1.len()))
            }
            fn parts_mut(&mut self) -> (&mut Matrix<T>, [usize; 4]) {
                let w = self.1.len();
                // SAFETY: see the note above.
                let m = unsafe { &mut *(self as *mut Self as *mut () as *mut Matrix<T>) };
                let win = unpack(m.0.shape(), w);
                (m, win)
            }
            /// Julia's `a[i, j] .= x` over the window: every entry set to `x`.
            pub fn fill(&mut self, x: T) {
                let (m, [r0, nr, c0, nc]) = self.parts_mut();
                for i in r0..r0 + nr { for j in c0..c0 + nc { m.0[(i, j)] = x; } }
            }
        }
    };
}
view_type!(
    /// `&a[0..2, ..]`: a borrowed window onto a matrix. Nothing is copied.
    View);
view_type!(
    /// `&a[1, ..]` or `&a[.., 2]`: a borrowed row or column, read as a vector.
    VectorView);

impl<T: Scalar + Copy> View<T> {
    pub fn nrows(&self) -> usize { self.parts().1[1] }
    pub fn ncols(&self) -> usize { self.parts().1[3] }
    pub fn size(&self) -> (usize, usize) { (self.nrows(), self.ncols()) }
    /// Julia's `a[1:2, :]` proper: the window as a matrix of its own.
    pub fn copy(&self) -> Matrix<T> {
        let (m, [r0, nr, c0, nc]) = self.parts();
        Matrix(m.0.view((r0, c0), (nr, nc)).into_owned())
    }
}
impl<T: Scalar + Copy> VectorView<T> {
    pub fn len(&self) -> usize { let [_, nr, _, nc] = self.parts().1; nr.max(nc) }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    /// Entry `k` of the row or column.
    pub(crate) fn entry(&self, k: usize) -> T {
        let (m, [r0, nr, c0, _]) = self.parts();
        if nr == 1 { m.0[(r0, c0 + k)] } else { m.0[(r0 + k, c0)] }
    }
    /// The row or column as a vector of its own.
    pub fn copy(&self) -> Vector<T> { Vector(DVector::from_iterator(self.len(), (0..self.len()).map(|k| self.entry(k)))) }
}
impl<T: Scalar + Copy + fmt::Display> fmt::Display for View<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (m, [r0, nr, c0, nc]) = self.parts();
        write!(f, "{nr}×{nc} view of Matrix<{}>:", short_type::<T>())?;
        for i in r0..r0 + nr {
            writeln!(f)?;
            for j in c0..c0 + nc { write!(f, " {}", m.0[(i, j)])?; }
        }
        Ok(())
    }
}
impl<T: Scalar + Copy + fmt::Display> fmt::Display for VectorView<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-element view of Matrix<{}>:", self.len(), short_type::<T>())?;
        for k in 0..self.len() { write!(f, "\n {}", self.entry(k))?; }
        Ok(())
    }
}

impl<T: Scalar + Copy, R: Span, C: Span> Index<(R, C)> for Matrix<T> {
    type Output = View<T>;
    #[track_caller]
    fn index(&self, rc: (R, C)) -> &View<T> {
        let shape = self.0.shape();
        View::of(self, pack(shape, rc.0.span(shape.0), rc.1.span(shape.1)))
    }
}
impl<T: Scalar + Copy, R: Span, C: Span> IndexMut<(R, C)> for Matrix<T> {
    #[track_caller]
    fn index_mut(&mut self, rc: (R, C)) -> &mut View<T> {
        let shape = self.0.shape();
        let w = pack(shape, rc.0.span(shape.0), rc.1.span(shape.1));
        View::of_mut(self, w)
    }
}
impl<T: Scalar + Copy, C: Span> Index<(usize, C)> for Matrix<T> {
    type Output = VectorView<T>;
    #[track_caller]
    fn index(&self, rc: (usize, C)) -> &VectorView<T> {
        let shape = self.0.shape();
        VectorView::of(self, pack(shape, (one(rc.0, shape.0), 1), rc.1.span(shape.1)))
    }
}
impl<T: Scalar + Copy, R: Span> Index<(R, usize)> for Matrix<T> {
    type Output = VectorView<T>;
    #[track_caller]
    fn index(&self, rc: (R, usize)) -> &VectorView<T> {
        let shape = self.0.shape();
        VectorView::of(self, pack(shape, rc.0.span(shape.0), (one(rc.1, shape.1), 1)))
    }
}
impl<T: Scalar + Copy, C: Span> IndexMut<(usize, C)> for Matrix<T> {
    #[track_caller]
    fn index_mut(&mut self, rc: (usize, C)) -> &mut VectorView<T> {
        let shape = self.0.shape();
        let w = pack(shape, (one(rc.0, shape.0), 1), rc.1.span(shape.1));
        VectorView::of_mut(self, w)
    }
}
impl<T: Scalar + Copy, R: Span> IndexMut<(R, usize)> for Matrix<T> {
    #[track_caller]
    fn index_mut(&mut self, rc: (R, usize)) -> &mut VectorView<T> {
        let shape = self.0.shape();
        let w = pack(shape, rc.0.span(shape.0), (one(rc.1, shape.1), 1));
        VectorView::of_mut(self, w)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Matrix, Vector};
    fn a() -> Matrix<i64> { Matrix::from_rows(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]) }

    #[test]
    fn slice_copies_and_an_integer_axis_is_dropped_as_in_julia() {
        let a = a();
        assert_eq!(a.slice(0..2, ..), Matrix::from_rows(vec![vec![1, 2, 3], vec![4, 5, 6]]));
        assert_eq!(a.slice(1.., ..=1), Matrix::from_rows(vec![vec![4, 5], vec![7, 8]]));
        assert_eq!(a.slice(..1, 1..=1), Matrix::from_rows(vec![vec![2]]));
        assert_eq!(a.slice(1, ..), Vector::from_vec(vec![4, 5, 6]));
        assert_eq!(a.slice(.., 2), Vector::from_vec(vec![3, 6, 9]));
        assert_eq!(a.slice(1, 1), 5);
        assert_eq!(Vector::from_vec(vec![1, 2, 3, 4]).slice(1..3), Vector::from_vec(vec![2, 3]));
    }

    #[test]
    fn an_index_with_ranges_is_a_view_of_the_same_entries() {
        let a = a();
        for (r, c) in [(0..2, 0..3), (1..3, 0..2), (0..0, 1..1), (2..3, 2..3), (0..3, 0..3)] {
            let v = &a[(r.clone(), c.clone())];
            assert_eq!(v.copy(), a.slice(r.clone(), c.clone()), "{r:?} {c:?}");
            assert_eq!(v.size(), (r.len(), c.len()));
        }
        assert_eq!(a[(1, ..)].copy(), a.slice(1, ..));
        assert_eq!(a[(.., 2)].copy(), a.slice(.., 2));
        assert_eq!(a[(1.., 0)].copy(), Vector::from_vec(vec![4, 7]));
        assert_eq!(a[(1, 1)], 5); // an element is still an element
        assert_eq!(std::mem::size_of::<&super::View<i64>>(), 2 * std::mem::size_of::<usize>());
        assert_eq!(format!("{}", &a[(0..2, 1..)]), "2×2 view of Matrix<i64>:\n 2 3\n 5 6");
        assert_eq!(format!("{}", &a[(2, ..)]), "3-element view of Matrix<i64>:\n 7\n 8\n 9");
    }

    #[test]
    fn a_window_is_packed_and_unpacked_exactly_for_every_window_of_a_matrix() {
        for shape in [(0, 0), (1, 1), (3, 5), (7, 2)] {
            for r0 in 0..=shape.0 { for nr in 0..=shape.0 - r0 { for c0 in 0..=shape.1 { for nc in 0..=shape.1 - c0 {
                assert_eq!(super::unpack(shape, super::pack(shape, (r0, nr), (c0, nc))), [r0, nr, c0, nc]);
            }}}}
        }
        // A million rows by a thousand columns still fits in one word.
        let big = (1_000_000, 1_000);
        assert_eq!(super::unpack(big, super::pack(big, (999_000, 1_000), (10, 990))), [999_000, 1_000, 10, 990]);
    }

    #[test]
    fn a_mutable_view_writes_through_to_the_matrix() {
        let mut a = a();
        a[(0..2, 1..)].fill(0);
        assert_eq!(a, Matrix::from_rows(vec![vec![1, 0, 0], vec![4, 0, 0], vec![7, 8, 9]]));
        a[(2, ..)].fill(-1);
        assert_eq!(a.slice(2, ..), Vector::from_vec(vec![-1, -1, -1]));
    }

    #[test]
    #[should_panic(expected = "BoundsError")]
    fn a_range_past_the_end_is_refused_as_julia_refuses_it() { let _ = a().slice(0..4, ..); }

    #[test]
    #[should_panic(expected = "BoundsError")]
    fn so_is_a_view_past_the_end() { let a = a(); let _ = &a[(.., 2..5)]; }
}
