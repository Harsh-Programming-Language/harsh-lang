//! Broadcasting: Julia's dotted operators and `f.(a)`.
//!
//! Harsh writes them `a .* b` and `f<> a`. Neither needs the transpiler to
//! know a type, or to parse an expression:
//!
//! - `a .* b` is emitted `a * hrs_std::DOT * b`. `a * DOT` is half an
//!   operation and `* b` completes it. Two operators of one level, so Rust's
//!   precedence and associativity are exactly Julia's: `x + a .* b` multiplies
//!   first, `a * b .* c` is `(a * b) .* c`.
//! - `f<> a b` is emitted `hrs_std::each!(f, a, b)`, which borrows every
//!   argument: applying a function to each element never consumes anything.
//!
//! Shapes stretch as Julia's do: along an axis two lengths must be equal, or
//! one of them 1. A number is 1×1, a vector n×1.

use crate::slicing::{VectorView, View};
use crate::{mismatch, Matrix, Vector};
use nalgebra::{DMatrix, DVector, Scalar};
use std::marker::PhantomData;
use std::ops::{Add, Div, Mul, Sub};

/// The kinds of operand: a number, a vector, a matrix.
pub struct KS;
pub struct KV;
pub struct KM;

/// Anything that can stand on one side of an elementwise operation.
pub trait Operand {
    type Elem: Scalar + Copy;
    type Kind;
    fn dims(&self) -> (usize, usize);
    fn get(&self, i: usize, j: usize) -> Self::Elem;
}
impl<T: Scalar + Copy> Operand for Matrix<T> {
    type Elem = T; type Kind = KM;
    fn dims(&self) -> (usize, usize) { self.0.shape() }
    fn get(&self, i: usize, j: usize) -> T { self.0[(i, j)] }
}
impl<T: Scalar + Copy> Operand for Vector<T> {
    type Elem = T; type Kind = KV;
    fn dims(&self) -> (usize, usize) { (self.0.len(), 1) }
    fn get(&self, i: usize, _: usize) -> T { self.0[i] }
}
impl<T: Scalar + Copy> Operand for View<T> {
    type Elem = T; type Kind = KM;
    fn dims(&self) -> (usize, usize) { self.size() }
    fn get(&self, i: usize, j: usize) -> T { let (m, [r0, _, c0, _]) = self.parts(); m.0[(r0 + i, c0 + j)] }
}
impl<T: Scalar + Copy> Operand for VectorView<T> {
    type Elem = T; type Kind = KV;
    fn dims(&self) -> (usize, usize) { (self.len(), 1) }
    fn get(&self, i: usize, _: usize) -> T { self.entry(i) }
}
impl<'a, X: Operand + ?Sized> Operand for &'a X {
    type Elem = X::Elem; type Kind = X::Kind;
    fn dims(&self) -> (usize, usize) { (**self).dims() }
    fn get(&self, i: usize, j: usize) -> X::Elem { (**self).get(i, j) }
}

/// What two kinds make together -- Julia's result shapes.
pub trait Shape<T: Scalar, K> {
    type Out;
    fn build(d: DMatrix<T>) -> Self::Out;
}
fn mat<T: Scalar>(d: DMatrix<T>) -> Matrix<T> { Matrix(d) }
fn col<T: Scalar + Copy>(d: DMatrix<T>) -> Vector<T> { Vector(DVector::from_iterator(d.nrows(), d.column(0).iter().copied())) }
fn num<T: Scalar + Copy>(d: DMatrix<T>) -> T { d[(0, 0)] }
macro_rules! shapes {
    ($($a:ty, $b:ty => $out:ty, $mk:ident;)*) => {$(
        impl<T: Scalar + Copy> Shape<T, $b> for $a { type Out = $out; fn build(d: DMatrix<T>) -> $out { $mk(d) } }
    )*};
}
shapes! {
    KM, KM => Matrix<T>, mat; KM, KS => Matrix<T>, mat; KS, KM => Matrix<T>, mat; KM, KV => Matrix<T>, mat; KV, KM => Matrix<T>, mat;
    KV, KV => Vector<T>, col; KV, KS => Vector<T>, col; KS, KV => Vector<T>, col;
    KS, KS => T, num;
}

/// The kind two operands make together, so a third can join it.
pub trait Join<K> { type Kind; }
macro_rules! joins { ($($a:ty, $b:ty => $k:ty;)*) => {$( impl Join<$b> for $a { type Kind = $k; } )*}; }
joins! { KM, KM => KM; KM, KV => KM; KV, KM => KM; KM, KS => KM; KS, KM => KM; KV, KV => KV; KV, KS => KV; KS, KV => KV; KS, KS => KS; }

#[track_caller]
fn axis(a: usize, b: usize) -> usize {
    if a != b && a != 1 && b != 1 {
        mismatch(format!("arrays could not be broadcast to a common size; got a dimension with lengths {a} and {b}"));
    }
    if a == 1 { b } else { a }
}

/// `f<> a`: `f` applied to each element. The result keeps `a`'s kind.
#[track_caller]
pub fn each1<L: Operand, U: Scalar + Copy>(f: impl Fn(L::Elem) -> U, l: L) -> <L::Kind as Shape<U, KS>>::Out
where L::Kind: Shape<U, KS> {
    let (nr, nc) = l.dims();
    <L::Kind as Shape<U, KS>>::build(DMatrix::from_fn(nr, nc, |i, j| f(l.get(i, j))))
}
/// `f<> a b`: `f` applied to each pair of elements, the shapes stretched.
#[track_caller]
pub fn each2<L: Operand, R: Operand, U: Scalar + Copy>(f: impl Fn(L::Elem, R::Elem) -> U, l: L, r: R) -> <L::Kind as Shape<U, R::Kind>>::Out
where L::Kind: Shape<U, R::Kind> {
    let ((lr, lc), (rr, rc)) = (l.dims(), r.dims());
    let (nr, nc) = (axis(lr, rr), axis(lc, rc));
    let at = |n: usize, i: usize| if n == 1 { 0 } else { i };
    <L::Kind as Shape<U, R::Kind>>::build(DMatrix::from_fn(nr, nc, |i, j| f(l.get(at(lr, i), at(lc, j)), r.get(at(rr, i), at(rc, j)))))
}
/// `f<> a b c`: three operands, stretched together.
#[track_caller]
pub fn each3<A: Operand, B: Operand, C: Operand, U: Scalar + Copy>(f: impl Fn(A::Elem, B::Elem, C::Elem) -> U, a: A, b: B, c: C)
    -> <<A::Kind as Join<B::Kind>>::Kind as Shape<U, C::Kind>>::Out
where A::Kind: Join<B::Kind>, <A::Kind as Join<B::Kind>>::Kind: Shape<U, C::Kind> {
    let (da, db, dc) = (a.dims(), b.dims(), c.dims());
    let (nr, nc) = (axis(axis(da.0, db.0), dc.0), axis(axis(da.1, db.1), dc.1));
    let at = |n: usize, i: usize| if n == 1 { 0 } else { i };
    <<A::Kind as Join<B::Kind>>::Kind as Shape<U, C::Kind>>::build(DMatrix::from_fn(nr, nc, |i, j| {
        f(a.get(at(da.0, i), at(da.1, j)), b.get(at(db.0, i), at(db.1, j)), c.get(at(dc.0, i), at(dc.1, j)))
    }))
}
/// What `f<> a b` is emitted as. Every argument is borrowed.
#[macro_export]
macro_rules! each {
    ($f:expr, $a:expr) => { $crate::each1($f, &($a)) };
    ($f:expr, $a:expr, $b:expr) => { $crate::each2($f, &($a), &($b)) };
    ($f:expr, $a:expr, $b:expr, $c:expr) => { $crate::each3($f, &($a), &($b), &($c)) };
    ($f:expr) => { compile_error!("`f<>` applies `f` to each element of something: `f<> a`, `f<> a b` or `f<> a b c`") };
    ($f:expr, $($rest:expr),+) => { compile_error!("`f<>` takes one, two or three arguments; for more, apply a closure to three and close over the rest") };
}

impl<T: Scalar + Copy> Matrix<T> {
    /// `a <- map f64.sqrt`: Julia's `map`, and the method beneath `f<> a`.
    pub fn map<U: Scalar + Copy>(&self, f: impl Fn(T) -> U) -> Matrix<U> { each1(f, self) }
}
impl<T: Scalar + Copy> Vector<T> {
    /// `v <- map f64.sqrt`.
    pub fn map<U: Scalar + Copy>(&self, f: impl Fn(T) -> U) -> Vector<U> { each1(f, self) }
}

/// What the `.` of `a .* b` is emitted as.
pub struct Dot;
pub const DOT: Dot = Dot;
/// `a * DOT`: the left half of an elementwise operation.
pub struct Half<L, O>(L, PhantomData<O>);

macro_rules! ops {
    ($($Tr:ident $m:ident $Tag:ident;)*) => {$(
        #[doc(hidden)] pub struct $Tag;
        impl<T: Scalar + Copy> $Tr<Dot> for Matrix<T> { type Output = Half<Matrix<T>, $Tag>; fn $m(self, _: Dot) -> Self::Output { Half(self, PhantomData) } }
        impl<'a, T: Scalar + Copy> $Tr<Dot> for &'a Matrix<T> { type Output = Half<&'a Matrix<T>, $Tag>; fn $m(self, _: Dot) -> Self::Output { Half(self, PhantomData) } }
        impl<T: Scalar + Copy> $Tr<Dot> for Vector<T> { type Output = Half<Vector<T>, $Tag>; fn $m(self, _: Dot) -> Self::Output { Half(self, PhantomData) } }
        impl<'a, T: Scalar + Copy> $Tr<Dot> for &'a Vector<T> { type Output = Half<&'a Vector<T>, $Tag>; fn $m(self, _: Dot) -> Self::Output { Half(self, PhantomData) } }
        impl<'a, T: Scalar + Copy> $Tr<Dot> for &'a View<T> { type Output = Half<&'a View<T>, $Tag>; fn $m(self, _: Dot) -> Self::Output { Half(self, PhantomData) } }
        impl<'a, T: Scalar + Copy> $Tr<Dot> for &'a VectorView<T> { type Output = Half<&'a VectorView<T>, $Tag>; fn $m(self, _: Dot) -> Self::Output { Half(self, PhantomData) } }
        impl<L: Operand, R: Operand<Elem = L::Elem>> $Tr<R> for Half<L, $Tag>
        where L::Kind: Shape<L::Elem, R::Kind>, L::Elem: $Tr<Output = L::Elem> {
            type Output = <L::Kind as Shape<L::Elem, R::Kind>>::Out;
            #[track_caller]
            fn $m(self, r: R) -> Self::Output { each2(|x: L::Elem, y: L::Elem| x.$m(y), self.0, r) }
        }
    )*};
}
ops! { Mul mul MulTag; Add add AddTag; Sub sub SubTag; Div div DivTag; }

// A number on either side. Rust's orphan rule wants each type named.
macro_rules! numbers {
    ($($t:ty)*) => {$(
        impl Operand for $t { type Elem = $t; type Kind = KS; fn dims(&self) -> (usize, usize) { (1, 1) } fn get(&self, _: usize, _: usize) -> $t { *self } }
        impl Mul<Dot> for $t { type Output = Half<$t, MulTag>; fn mul(self, _: Dot) -> Self::Output { Half(self, PhantomData) } }
        impl Add<Dot> for $t { type Output = Half<$t, AddTag>; fn add(self, _: Dot) -> Self::Output { Half(self, PhantomData) } }
        impl Sub<Dot> for $t { type Output = Half<$t, SubTag>; fn sub(self, _: Dot) -> Self::Output { Half(self, PhantomData) } }
        impl Div<Dot> for $t { type Output = Half<$t, DivTag>; fn div(self, _: Dot) -> Self::Output { Half(self, PhantomData) } }
    )*};
}
numbers!(f64 f32 i64 i32 usize bool);

#[cfg(test)]
mod tests {
    use crate::{Matrix, Vector, DOT};
    fn m(rows: Vec<Vec<f64>>) -> Matrix<f64> { Matrix::from_rows(rows) }
    fn ab() -> (Matrix<f64>, Matrix<f64>) { (m(vec![vec![1.0, 2.0], vec![3.0, 4.0]]), m(vec![vec![10.0, 20.0], vec![30.0, 40.0]])) }

    #[test]
    fn a_dotted_operator_is_elementwise_and_keeps_julias_precedence() {
        let (a, b) = ab();
        assert_eq!(&a * DOT * &b, m(vec![vec![10.0, 40.0], vec![90.0, 160.0]]));
        assert_ne!(&a * DOT * &b, &a * &b);
        // x + a .* b: the elementwise product first.
        assert_eq!(&a + &a * DOT * &b, m(vec![vec![11.0, 42.0], vec![93.0, 164.0]]));
        // a * b .* a is (a * b) .* a, as in Julia.
        assert_eq!(&a * &b * DOT * &a, m(vec![vec![70.0, 200.0], vec![450.0, 880.0]]));
        assert_eq!(&b / DOT / &a, m(vec![vec![10.0, 10.0], vec![10.0, 10.0]]));
        assert_eq!(2.0 * DOT * &a + DOT + 1.0, m(vec![vec![3.0, 5.0], vec![7.0, 9.0]]));
        assert_eq!(a.clone() - DOT - 1.0, m(vec![vec![0.0, 1.0], vec![2.0, 3.0]]));
    }

    #[test]
    fn shapes_stretch_as_julias_do() {
        let (a, _) = ab();
        let v = Vector::from_vec(vec![1.0, 2.0]);
        assert_eq!(&a - DOT - &v, m(vec![vec![0.0, 1.0], vec![1.0, 2.0]]));
        let row = m(vec![vec![100.0, 200.0]]);
        assert_eq!(&a + DOT + &row, m(vec![vec![101.0, 202.0], vec![103.0, 204.0]]));
        // A column against a row: the outer sum.
        assert_eq!(&v + DOT + &row, m(vec![vec![101.0, 201.0], vec![102.0, 202.0]]));
        assert_eq!(&v * DOT * &v, Vector::from_vec(vec![1.0, 4.0]));
        assert_eq!(&v * DOT * 3.0, Vector::from_vec(vec![3.0, 6.0]));
        // A view is an operand like any other.
        assert_eq!(&a[(0..1, ..)] * DOT * &row, m(vec![vec![100.0, 400.0]]));
        assert_eq!(&a[(.., 1)] * DOT * &v, Vector::from_vec(vec![2.0, 8.0]));
    }

    #[test]
    fn each_applies_a_function_and_borrows_every_argument() {
        let (a, b) = ab();
        assert_eq!(crate::each!(f64::sqrt, m(vec![vec![4.0, 9.0]])), m(vec![vec![2.0, 3.0]]));
        assert_eq!(crate::each!(|x| x * x, a), m(vec![vec![1.0, 4.0], vec![9.0, 16.0]]));
        // `powf` is not exact, and Miri perturbs it on purpose to catch a
        // test that assumes so (the user's Miri run, 2026-09-25: 8.999999999999995):
        // compared within a tolerance.
        let p: Matrix<f64> = crate::each!(f64::powf, a, 2.0);
        let want = [[1.0, 4.0], [9.0, 16.0]];
        for i in 0..2 {
            for j in 0..2 {
                assert!((p.0[(i, j)] - want[i][j]).abs() < 1e-9, "{:?}", p);
            }
        }
        assert_eq!(crate::each!(f64::max, a, b), b);
        // Julia's `b .> 25`: the element type may change.
        assert_eq!(crate::each!(|x, y| x > y, b, 25.0), Matrix::from_rows(vec![vec![false, false], vec![true, true]]));
        assert_eq!(crate::each!(|x: f64| x as i64, Vector::from_vec(vec![1.5, 2.5])), Vector::from_vec(vec![1, 2]));
        assert_eq!(crate::each!(f64::sqrt, 16.0), 4.0);
        // Three operands: Julia's `clamp.(a, lo, hi)` and `muladd.(a, b, c)`.
        assert_eq!(crate::each!(f64::clamp, a, 2.0, 3.0), m(vec![vec![2.0, 2.0], vec![3.0, 3.0]]));
        let v = Vector::from_vec(vec![100.0, 200.0]);
        assert_eq!(crate::each!(f64::mul_add, a, b, v), m(vec![vec![110.0, 140.0], vec![290.0, 360.0]]));
        assert_eq!(crate::each!(|x: f64, y: f64, z: f64| x + y + z, v, 1.0, 2.0), Vector::from_vec(vec![103.0, 203.0]));
        assert_eq!(a.map(|x| x + 1.0), &a + DOT + 1.0);
        let _still_here = (a, b); // nothing above moved them
    }

    #[test]
    #[should_panic(expected = "DimensionMismatch: arrays could not be broadcast to a common size; got a dimension with lengths 2 and 3")]
    fn shapes_that_do_not_stretch_are_refused_in_julias_words() {
        let _ = &ab().0 * DOT * &m(vec![vec![1.0, 2.0, 3.0]]);
    }
}
