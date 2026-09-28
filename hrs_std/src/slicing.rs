//! Slicing: Julia's `a[1:2, :]`, in Harsh's own ranges and 0-based.
//!
//! `..` alone is Julia's `:`. Two spellings, one meaning (ruled 2026-09-21):
//!
//! - **the method**, `a <- slice (0..2) (..)` -- a copy, as Julia's is;
//! - **the view**, `a <- view (0..2) (..)` -- a borrowed window, nothing
//!   copied, as Julia's `view(a, 1:2, :)` is; `a <- view_mut …` to write
//!   through it; `<- copy$` for a copy of it. (Until 0.1.37 a view was
//!   written `&a[0..2, ..]`, an index; that form is gone -- see "views".)
//!
//! One element is still an index: `a[i, j]`, and `a[i, j] = x` (written with
//! a comma in Harsh; the transpiler makes the tuple Rust wants).
//!
//! As in Julia, an axis taken by an integer is dropped: `a[1, ..]` is a
//! vector, `a[1, 2]` a number. Harsh has no types; the trait system picks.

use crate::{short_type, Matrix, Vector};
use nalgebra::{DVector, Scalar};
use std::fmt;
use std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

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
// A view is a value: a reference to the matrix and the window it looks at.
// Nothing is copied, and the borrow checker guards it as it guards any
// borrow -- a view cannot outlive its matrix, nor be held while the matrix
// is written. There is no `unsafe` here (0.1.37).
//
// Why a method and not `&a[0..2, ..]` any more: Rust's `Index` must return a
// *reference*, and a window is not a value stored in the matrix to refer to.
// Until 0.1.36, this crate made one up -- a zero-sized reference to the matrix
// with the window packed into its length -- which Miri found undefined
// behaviour under Stacked Borrows. The remedy, the user's choice
// (2026-09-25, VIEWS-DESIGN): a view as a value, with a spelling of its own.

/// `a <- view (0..2) (..)`: a borrowed window onto a matrix.
#[derive(Clone, Copy)]
pub struct View<'a, T: Scalar> { m: &'a Matrix<T>, r0: usize, nr: usize, c0: usize, nc: usize }
/// `a <- view_mut (0..2) (..)`: a window that writes through to the matrix.
pub struct ViewMut<'a, T: Scalar> { m: &'a mut Matrix<T>, r0: usize, nr: usize, c0: usize, nc: usize }
/// `a <- view 1 (..)` or `a <- view (..) 2`: one row or column, read as a vector.
#[derive(Clone, Copy)]
pub struct VectorView<'a, T: Scalar> { m: &'a Matrix<T>, r0: usize, nr: usize, c0: usize, nc: usize }
/// `a <- view_mut 1 (..)`: one row or column, written through.
pub struct VectorViewMut<'a, T: Scalar> { m: &'a mut Matrix<T>, r0: usize, nr: usize, c0: usize, nc: usize }

/// What a pair of axes views in a matrix, by the types of the pair: a
/// window, a row or column, or one element -- as `Pick` decides for `slice`.
pub trait ViewPick<'a, T: Scalar> {
    type Out;
    fn view_of(self, m: &'a Matrix<T>) -> Self::Out;
}
/// The same, to write through.
pub trait ViewMutPick<'a, T: Scalar> {
    type Out;
    fn view_mut_of(self, m: &'a mut Matrix<T>) -> Self::Out;
}
impl<'a, T: Scalar + Copy, R: Span, C: Span> ViewPick<'a, T> for (R, C) {
    type Out = View<'a, T>;
    #[track_caller]
    fn view_of(self, m: &'a Matrix<T>) -> View<'a, T> {
        let ((r0, nr), (c0, nc)) = (self.0.span(m.0.nrows()), self.1.span(m.0.ncols()));
        View { m, r0, nr, c0, nc }
    }
}
impl<'a, T: Scalar + Copy, C: Span> ViewPick<'a, T> for (usize, C) {
    type Out = VectorView<'a, T>;
    #[track_caller]
    fn view_of(self, m: &'a Matrix<T>) -> VectorView<'a, T> {
        let (r0, (c0, nc)) = (one(self.0, m.0.nrows()), self.1.span(m.0.ncols()));
        VectorView { m, r0, nr: 1, c0, nc }
    }
}
impl<'a, T: Scalar + Copy, R: Span> ViewPick<'a, T> for (R, usize) {
    type Out = VectorView<'a, T>;
    #[track_caller]
    fn view_of(self, m: &'a Matrix<T>) -> VectorView<'a, T> {
        let ((r0, nr), c0) = (self.0.span(m.0.nrows()), one(self.1, m.0.ncols()));
        VectorView { m, r0, nr, c0, nc: 1 }
    }
}
impl<'a, T: Scalar + Copy> ViewPick<'a, T> for (usize, usize) {
    type Out = &'a T;
    #[track_caller]
    fn view_of(self, m: &'a Matrix<T>) -> &'a T { &m.0[(one(self.0, m.0.nrows()), one(self.1, m.0.ncols()))] }
}
impl<'a, T: Scalar + Copy, R: Span, C: Span> ViewMutPick<'a, T> for (R, C) {
    type Out = ViewMut<'a, T>;
    #[track_caller]
    fn view_mut_of(self, m: &'a mut Matrix<T>) -> ViewMut<'a, T> {
        let ((r0, nr), (c0, nc)) = (self.0.span(m.0.nrows()), self.1.span(m.0.ncols()));
        ViewMut { m, r0, nr, c0, nc }
    }
}
impl<'a, T: Scalar + Copy, C: Span> ViewMutPick<'a, T> for (usize, C) {
    type Out = VectorViewMut<'a, T>;
    #[track_caller]
    fn view_mut_of(self, m: &'a mut Matrix<T>) -> VectorViewMut<'a, T> {
        let (r0, (c0, nc)) = (one(self.0, m.0.nrows()), self.1.span(m.0.ncols()));
        VectorViewMut { m, r0, nr: 1, c0, nc }
    }
}
impl<'a, T: Scalar + Copy, R: Span> ViewMutPick<'a, T> for (R, usize) {
    type Out = VectorViewMut<'a, T>;
    #[track_caller]
    fn view_mut_of(self, m: &'a mut Matrix<T>) -> VectorViewMut<'a, T> {
        let ((r0, nr), c0) = (self.0.span(m.0.nrows()), one(self.1, m.0.ncols()));
        VectorViewMut { m, r0, nr, c0, nc: 1 }
    }
}
impl<'a, T: Scalar + Copy> ViewMutPick<'a, T> for (usize, usize) {
    type Out = &'a mut T;
    #[track_caller]
    fn view_mut_of(self, m: &'a mut Matrix<T>) -> &'a mut T {
        let (i, j) = (one(self.0, m.0.nrows()), one(self.1, m.0.ncols()));
        &mut m.0[(i, j)]
    }
}

impl<T: Scalar + Copy> Matrix<T> {
    /// `a <- view (0..2) (..)`: Julia's `view(a, 1:2, :)` -- a window onto
    /// the matrix, borrowed; nothing is copied. An integer axis gives a row
    /// or column (`a <- view 1 (..)`), two integers one element.
    #[track_caller]
    pub fn view<'a, R, C>(&'a self, rows: R, cols: C) -> <(R, C) as ViewPick<'a, T>>::Out
    where (R, C): ViewPick<'a, T> {
        (rows, cols).view_of(self)
    }
    /// `a <- view_mut (0..2) (..)`: the same window, writing through to the
    /// matrix -- `<- fill 0`, or an element `x <- view_mut 1 2` then `*x = 5`.
    #[track_caller]
    pub fn view_mut<'a, R, C>(&'a mut self, rows: R, cols: C) -> <(R, C) as ViewMutPick<'a, T>>::Out
    where (R, C): ViewMutPick<'a, T> {
        (rows, cols).view_mut_of(self)
    }
}

// What every window can do, borrowed or writing.
macro_rules! window {
    ($V:ident) => {
        impl<'a, T: Scalar + Copy> $V<'a, T> {
            pub fn nrows(&self) -> usize { self.nr }
            pub fn ncols(&self) -> usize { self.nc }
            pub fn size(&self) -> (usize, usize) { (self.nr, self.nc) }
            /// Entry `(i, j)` of the window.
            #[track_caller]
            pub fn get(&self, i: usize, j: usize) -> T { self.m.0[(self.r0 + one(i, self.nr), self.c0 + one(j, self.nc))] }
            /// Julia's `a[1:2, :]` proper: the window as a matrix of its own.
            pub fn copy(&self) -> Matrix<T> { Matrix(self.m.0.view((self.r0, self.c0), (self.nr, self.nc)).into_owned()) }
        }
        impl<'a, T: Scalar + Copy + fmt::Display> fmt::Display for $V<'a, T> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}×{} view of Matrix<{}>:", self.nr, self.nc, short_type::<T>())?;
                for i in self.r0..self.r0 + self.nr {
                    writeln!(f)?;
                    for j in self.c0..self.c0 + self.nc { write!(f, " {}", self.m.0[(i, j)])?; }
                }
                Ok(())
            }
        }
    };
}
window!(View);
window!(ViewMut);

// What every row or column can do.
macro_rules! line {
    ($V:ident) => {
        impl<'a, T: Scalar + Copy> $V<'a, T> {
            pub fn len(&self) -> usize { self.nr.max(self.nc) }
            pub fn is_empty(&self) -> bool { self.nr == 0 || self.nc == 0 }
            /// Entry `k` of the row or column.
            #[track_caller]
            pub fn get(&self, k: usize) -> T {
                let k = one(k, self.len());
                if self.nr == 1 { self.m.0[(self.r0, self.c0 + k)] } else { self.m.0[(self.r0 + k, self.c0)] }
            }
            /// The row or column as a vector of its own.
            pub fn copy(&self) -> Vector<T> { Vector(DVector::from_iterator(self.len(), (0..self.len()).map(|k| self.get(k)))) }
        }
        impl<'a, T: Scalar + Copy + fmt::Display> fmt::Display for $V<'a, T> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}-element view of Matrix<{}>:", self.len(), short_type::<T>())?;
                for k in 0..self.len() { write!(f, "\n {}", self.get(k))?; }
                Ok(())
            }
        }
    };
}
line!(VectorView);
line!(VectorViewMut);

impl<'a, T: Scalar + Copy> ViewMut<'a, T> {
    /// Julia's `a[i, j] .= x` over the window: every entry set to `x`.
    pub fn fill(&mut self, x: T) {
        for i in self.r0..self.r0 + self.nr { for j in self.c0..self.c0 + self.nc { self.m.0[(i, j)] = x; } }
    }
    /// Entry `(i, j)` of the window, to write: `*w <- at 0 1 = 5`.
    #[track_caller]
    pub fn at(&mut self, i: usize, j: usize) -> &mut T {
        let (i, j) = (self.r0 + one(i, self.nr), self.c0 + one(j, self.nc));
        &mut self.m.0[(i, j)]
    }
}
impl<'a, T: Scalar + Copy> VectorViewMut<'a, T> {
    /// Every entry of the row or column set to `x`.
    pub fn fill(&mut self, x: T) {
        for i in self.r0..self.r0 + self.nr { for j in self.c0..self.c0 + self.nc { self.m.0[(i, j)] = x; } }
    }
    /// Entry `k`, to write.
    #[track_caller]
    pub fn at(&mut self, k: usize) -> &mut T {
        let k = one(k, self.len());
        let (i, j) = if self.nr == 1 { (self.r0, self.c0 + k) } else { (self.r0 + k, self.c0) };
        &mut self.m.0[(i, j)]
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
    fn a_view_is_the_same_entries_as_a_slice_nothing_copied() {
        let a = a();
        for (r, c) in [(0..2, 0..3), (1..3, 0..2), (0..0, 1..1), (2..3, 2..3), (0..3, 0..3)] {
            let v = a.view(r.clone(), c.clone());
            assert_eq!(v.copy(), a.slice(r.clone(), c.clone()), "{r:?} {c:?}");
            assert_eq!(v.size(), (r.len(), c.len()));
        }
        assert_eq!(a.view(1, ..).copy(), a.slice(1, ..));
        assert_eq!(a.view(.., 2).copy(), a.slice(.., 2));
        assert_eq!(a.view(1.., 0).copy(), Vector::from_vec(vec![4, 7]));
        assert_eq!(*a.view(1, 1), 5);
        assert_eq!(a.view(1.., ..).get(1, 2), 9);
        assert_eq!(a.view(.., 1).get(2), 8);
        assert_eq!(format!("{}", a.view(0..2, 1..)), "2×2 view of Matrix<i64>:\n 2 3\n 5 6");
        assert_eq!(format!("{}", a.view(2, ..)), "3-element view of Matrix<i64>:\n 7\n 8\n 9");
    }

    #[test]
    fn a_view_is_a_value_with_no_unsafe_behind_it() {
        // A reference and four numbers: nothing packed, nothing forged.
        assert_eq!(std::mem::size_of::<super::View<i64>>(), std::mem::size_of::<&Matrix<i64>>() + 4 * std::mem::size_of::<usize>());
        // Views are values: they copy, and several can look at once.
        let a = a();
        let (top, left) = (a.view(0..1, ..), a.view(.., 0..1));
        let again = top;
        assert_eq!((top.copy(), left.copy(), again.size()), (a.slice(0..1, ..), a.slice(.., 0..1), (1, 3)));
    }

    #[test]
    fn a_mutable_view_writes_through_to_the_matrix() {
        let mut a = a();
        a.view_mut(0..2, 1..).fill(0);
        assert_eq!(a, Matrix::from_rows(vec![vec![1, 0, 0], vec![4, 0, 0], vec![7, 8, 9]]));
        a.view_mut(2, ..).fill(-1);
        assert_eq!(a.slice(2, ..), Vector::from_vec(vec![-1, -1, -1]));
        *a.view_mut(1, 1) = 42;
        *a.view_mut(0.., 0..).at(2, 2) = 7;
        *a.view_mut(.., 0).at(0) = 100;
        assert_eq!(a, Matrix::from_rows(vec![vec![100, 0, 0], vec![4, 42, 0], vec![-1, -1, 7]]));
    }

    #[test]
    #[should_panic(expected = "BoundsError")]
    fn a_range_past_the_end_is_refused_as_julia_refuses_it() { let _ = a().slice(0..4, ..); }

    #[test]
    #[should_panic(expected = "BoundsError")]
    fn so_is_a_view_past_the_end() { let a = a(); let _ = a.view(.., 2..5); }

    #[test]
    #[should_panic(expected = "BoundsError")]
    fn and_an_entry_past_a_view_s_end() { let a = a(); let _ = a.view(0..2, 0..2).get(2, 0); }
}
