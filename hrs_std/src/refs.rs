//! Every borrowing form of `+`, `-` and `*`, and assignment to an element.
//!
//! 0.1.0 had `&a + &b` and `a + b` but not `&a + b`, so `&a + &a * &b` -- whose
//! right side is a fresh matrix -- did not compile; and it had no `IndexMut`,
//! so `a[(1, 1)] = 5.0` did not either. Both found by the slicing prototypes
//! (2026-09-21). An owned operand is simply borrowed: nothing is cloned.

use crate::{check_product, check_same, Matrix, Vector};
use nalgebra::{DMatrix, DVector, Scalar};
use std::ops::{Add, IndexMut, Mul, Sub};

macro_rules! same_shape {
    ($Tr:ident $m:ident $W:ident $D:ident; $( [$($l:tt)*] [$($r:tt)*] ),*) => {$(
        impl<'a, T: Scalar> $Tr<$($r)* $W<T>> for $($l)* $W<T>
        where for<'x> &'x $D<T>: $Tr<&'x $D<T>, Output = $D<T>> {
            type Output = $W<T>;
            #[track_caller]
            fn $m(self, rhs: $($r)* $W<T>) -> $W<T> {
                check_same(self.0.shape(), rhs.0.shape());
                $W((&self.0).$m(&rhs.0))
            }
        }
    )*};
}
same_shape!(Add add Matrix DMatrix; [&'a] [], [] [&'a]);
same_shape!(Sub sub Matrix DMatrix; [&'a] [], [] [&'a], [&'a] [&'a]);
same_shape!(Add add Vector DVector; [&'a] [], [] [&'a], [&'a] [&'a]);
same_shape!(Sub sub Vector DVector; [&'a] [], [] [&'a]);

macro_rules! product {
    ($R:ident $DR:ident; $( [$($l:tt)*] [$($r:tt)*] ),*) => {$(
        impl<'a, T: Scalar> Mul<$($r)* $R<T>> for $($l)* Matrix<T>
        where for<'x> &'x DMatrix<T>: Mul<&'x $DR<T>, Output = $DR<T>> {
            type Output = $R<T>;
            #[track_caller]
            fn mul(self, rhs: $($r)* $R<T>) -> $R<T> {
                check_product(self.0.shape(), rhs.0.shape());
                $R(&self.0 * &rhs.0)
            }
        }
    )*};
}
product!(Matrix DMatrix; [&'a] [], [] [&'a]);
product!(Vector DVector; [&'a] [], [] [&'a]);

/// `a[(i, j)] = x`, 0-based.
impl<T: Scalar> IndexMut<(usize, usize)> for Matrix<T> {
    #[track_caller]
    fn index_mut(&mut self, ij: (usize, usize)) -> &mut T { &mut self.0[ij] }
}
/// `v[i] = x`, 0-based.
impl<T: Scalar> IndexMut<usize> for Vector<T> {
    #[track_caller]
    fn index_mut(&mut self, i: usize) -> &mut T { &mut self.0[i] }
}

#[cfg(test)]
mod tests {
    use crate::{Matrix, Vector};
    fn m(rows: Vec<Vec<f64>>) -> Matrix<f64> { Matrix::from_rows(rows) }

    #[test]
    fn every_borrowing_form_of_an_operator_compiles_and_agrees() {
        let a = m(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let b = m(vec![vec![10.0, 20.0], vec![30.0, 40.0]]);
        let sum = a.clone() + b.clone();
        assert_eq!(&a + b.clone(), sum);
        assert_eq!(a.clone() + &b, sum);
        // The case that did not compile: the right side is a fresh matrix.
        assert_eq!(&a + &a * &b, a.clone() + (a.clone() * b.clone()));
        let diff = a.clone() - b.clone();
        assert_eq!(&a - &b, diff);
        assert_eq!(&a - b.clone(), diff);
        assert_eq!(a.clone() - &b, diff);
        let prod = &a * &b;
        assert_eq!(&a * b.clone(), prod);
        assert_eq!(a.clone() * &b, prod);
        let v = Vector::from_vec(vec![1.0, 2.0]);
        assert_eq!(&a * v.clone(), &a * &v);
        assert_eq!(a.clone() * &v, &a * &v);
        assert_eq!(&v + &v, v.clone() + v.clone());
        assert_eq!(&v + v.clone(), v.clone() + &v);
        assert_eq!(&v - v.clone(), v.clone() - &v);
    }

    #[test]
    fn an_element_can_be_assigned() {
        let mut a = m(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        a[(1, 0)] = 30.0;
        a[(0, 1)] += 0.5;
        assert_eq!(a, m(vec![vec![1.0, 2.5], vec![30.0, 4.0]]));
        let mut v = Vector::from_vec(vec![1, 2, 3]);
        v[2] = 9;
        assert_eq!(v, Vector::from_vec(vec![1, 2, 9]));
    }

    #[test]
    #[should_panic(expected = "DimensionMismatch")]
    fn a_mixed_form_still_checks_its_shapes() {
        let _ = &m(vec![vec![1.0, 2.0]]) + m(vec![vec![1.0], vec![2.0]]);
    }
}
