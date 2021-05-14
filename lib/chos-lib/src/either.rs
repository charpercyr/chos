
use core::fmt::Debug;
use core::hint::unreachable_unchecked;
use core::pin::Pin;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

use Either::*;

impl<L, R> Either<L, R> {
    pub fn is_left(&self) -> bool {
        match self {
            Left(_) => true,
            Right(_) => false,
        }
    }

    pub fn is_right(&self) -> bool {
        match self {
            Left(_) => false,
            Right(_) => true,
        }
    }

    pub fn left_map<T, F: FnOnce(L) -> T>(self, f: F) -> Either<T, R> {
        match self {
            Left(l) => Left(f(l)),
            Right(r) => Right(r),
        }
    }

    pub fn left_map_or<T, F: FnOnce(L) -> T>(self, default: T, f: F) -> T {
        match self {
            Left(l) => f(l),
            Right(_) => default,
        }
    }

    pub fn left_map_or_else<T, D, F>(self, default: D, f: F) -> T
    where
        F: FnOnce(L) -> T,
        D: FnOnce(R) -> T,
    {
        match self {
            Left(l) => f(l),
            Right(r) => default(r),
        }
    }

    pub fn right_map<T, F: FnOnce(R) -> T>(self, f: F) -> Either<L, T> {
        match self {
            Left(l) => Left(l),
            Right(r) => Right(f(r)),
        }
    }

    pub fn right_map_or<T, F: FnOnce(R) -> T>(self, default: T, f: F) -> T {
        match self {
            Left(_) => default,
            Right(r) => f(r),
        }
    }

    pub fn right_map_or_else<T, D, F>(self, default: D, f: F) -> T
    where
        F: FnOnce(R) -> T,
        D: FnOnce(L) -> T,
    {
        match self {
            Left(l) => default(l),
            Right(r) => f(r),
        }
    }

    pub fn left(self) -> Option<L> {
        match self {
            Left(l) => Some(l),
            _ => None,
        }
    }

    pub fn right(self) -> Option<R> {
        match self {
            Right(r) => Some(r),
            _ => None,
        }
    }

    pub fn unwrap_left(self) -> L where R: Debug {
        match self {
            Left(l) => l,
            Right(r) => panic!("Called Either::unwrap_left on a 'Right' value: {:?}", r),
        }
    }

    pub fn unwrap_left_or(self, default: L) -> L {
        match self {
            Left(l) => l,
            _ => default,
        }
    }

    pub fn unwrap_left_or_else(self, f: impl FnOnce() -> L) -> L {
        match self {
            Left(l) => l,
            _ => f(),
        }
    }

    pub unsafe fn unwrap_left_unchecked(self) -> L {
        match self {
            Left(l) => l,
            _ => unreachable_unchecked(),
        }
    }

    pub fn unwrap_right(self) -> R where L: Debug {
        match self {
            Right(r) => r,
            Left(l) => panic!("Called Either::unwrap_right on a 'Left' value: {:?}", l),
        }
    }

    pub fn unwrap_right_or(self, default: R) -> R {
        match self {
            Right(r) => r,
            _ => default,
        }
    }

    pub fn unwrap_right_or_else(self, f: impl FnOnce() -> R) -> R {
        match self {
            Right(r) => r,
            _ => f(),
        }
    }

    pub unsafe fn unwrap_right_unchecked(self) -> R {
        match self {
            Right(r) => r,
            _ => unreachable_unchecked(),
        }
    }

    pub fn expect_left(self, msg: &str) -> L where R: Debug {
        match self {
            Left(l) => l,
            Right(r) => panic!("{}: {:?}", msg, r),
        }
    }

    pub fn expect_right(self, msg: &str) -> R where L: Debug {
        match self {
            Right(r) => r,
            Left(l) => panic!("{}: {:?}", msg, l),
        }
    }

    pub fn contains_left<U: PartialEq<L>>(&self, u: &U) -> bool {
        match self {
            Left(l) => u.eq(l),
            _ => false,
        }
    }

    pub fn contains_right<U: PartialEq<R>>(&self, u: &U) -> bool {
        match self {
            Right(r) => u.eq(r),
            _ => false,
        }
    }

    pub fn as_ref(&self) -> Either<&L, &R> {
        match self {
            Left(l) => Left(l),
            Right(r) => Right(r),
        }
    }

    pub fn as_mut(&mut self) -> Either<&mut L, &mut R> {
        match self {
            Left(l) => Left(l),
            Right(r) => Right(r),
        }
    }

    pub fn as_pin_ref(self: Pin<&Self>) -> Either<Pin<&L>, Pin<&R>> {
        match self.get_ref() {
            Left(l) => unsafe { Left(Pin::new_unchecked(l)) },
            Right(r) => unsafe { Right(Pin::new_unchecked(r)) },
        }
    }

    pub fn as_pin_mut(self: Pin<&mut Self>) -> Either<Pin<&mut L>, Pin<&mut R>> {
        match unsafe { self.get_unchecked_mut() } {
            Left(l) => unsafe { Left(Pin::new_unchecked(l)) },
            Right(r) => unsafe { Right(Pin::new_unchecked(r)) },
        }
    }
}

impl<T> Either<T, T> {
    pub fn into_inner(self) -> T {
        match self {
            Left(l) => l,
            Right(r) => r,
        }
    }
}
