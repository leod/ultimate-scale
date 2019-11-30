use std::marker::PhantomData;
use std::ops::{Mul, RangeInclusive};

use num_traits::Float;

pub trait Fun<T, V> {
    fn eval(&self, t: T) -> V;
}

struct WrapFn<T, V, F: Fn(T) -> V>(F, PhantomData<(T, V)>);

impl<T, V, F> Fun<T, V> for WrapFn<T, V, F>
where
    T: Float,
    V: Float,
    F: Fn(T) -> V,
{
    fn eval(&self, t: T) -> V {
        self.0(t)
    }
}

pub struct Anim<T, V, F>(F, PhantomData<(T, V)>);

impl<T, V, F> Anim<T, V, F>
where
    T: Float,
    V: Float,
    F: Fun<T, V>,
{
    pub fn new(f: F) -> Self {
        Anim(f, PhantomData)
    }

    pub fn eval(&self, t: T) -> V {
        self.0.eval(t)
    }
}

impl<T, V, F> From<F> for Anim<T, V, WrapFn<T, V, F>>
where
    T: Float,
    V: Float,
    F: Fn(T) -> V,
{
    fn from(f: F) -> Self {
        Self(WrapFn(f, PhantomData), PhantomData)
    }
}

pub fn lift<T: Float, V: Float>(f: impl Fn(T) -> V) -> Anim<T, V, impl Fun<T, V>> {
    From::from(f)
}

pub fn squeeze<T, V, F>(
    range: RangeInclusive<T>,
    default: V,
    a: Anim<T, V, F>,
) -> Anim<T, V, impl Fun<T, V>>
where
    T: Float,
    V: Float,
    F: Fun<T, V>,
{
    Anim::from(move |t| {
        if range.contains(&t) {
            a.eval((t - *range.start()) * T::one() / (*range.end() - *range.start()))
        } else {
            default
        }
    })
}

pub fn constant<T: Float, V: Float>(c: V) -> Anim<T, V, impl Fun<T, V>> {
    Anim::from(move |_| c)
}

pub fn one<T: Float, V: Float>() -> Anim<T, V, impl Fun<T, V>> {
    constant(V::one())
}

pub fn zero<T: Float, V: Float>() -> Anim<T, V, impl Fun<T, V>> {
    constant(V::zero())
}

pub fn backwards<T: Float, V: Float>(
    end: T,
    a: Anim<T, V, impl Fun<T, V>>,
) -> Anim<T, V, impl Fun<T, V>> {
    Anim::from(move |t| a.eval(end - t))
}

impl<T, V, F> Mul<V> for Anim<T, V, F>
where
    T: Float,
    V: Float,
    F: Fun<T, V>,
{
    type Output = Anim<T, V, MulConstantClosure<T, V, F>>;

    fn mul(self, rhs: V) -> Self::Output {
        Anim::new(MulConstantClosure(self, rhs))
    }
}

pub struct MulConstantClosure<T, V, F>(Anim<T, V, F>, V);

impl<T, V, F> Fun<T, V> for MulConstantClosure<T, V, F>
where
    T: Float,
    V: Float,
    F: Fun<T, V>,
{
    fn eval(&self, t: T) -> V {
        self.0.eval(t) * self.1
    }
}

impl<T, V, F1, F2> Mul<Anim<T, V, F2>> for Anim<T, V, F1>
where
    T: Float,
    V: Float,
    F1: Fun<T, V>,
    F2: Fun<T, V>,
{
    type Output = Anim<T, V, MulClosure<T, V, F1, F2>>;

    fn mul(self, rhs: Anim<T, V, F2>) -> Self::Output {
        Anim::new(MulClosure(self, rhs))
    }
}

pub struct MulClosure<T, V, F1, F2>(Anim<T, V, F1>, Anim<T, V, F2>);

impl<T, V, F1, F2> Fun<T, V> for MulClosure<T, V, F1, F2>
where
    T: Float,
    V: Float,
    F1: Fun<T, V>,
    F2: Fun<T, V>,
{
    fn eval(&self, t: T) -> V {
        self.0.eval(t) * self.1.eval(t)
    }
}
