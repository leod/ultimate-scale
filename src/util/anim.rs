use std::marker::PhantomData;
use std::ops::{Add, Mul, RangeInclusive};

use num_traits::{Float, FloatConst};

pub trait Fun<T, V> {
    fn eval(&self, t: T) -> V;
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

    pub fn backwards(self, end: T) -> Anim<T, V, impl Fun<T, V>> {
        Anim::from(move |t| self.eval(end - t))
    }

    pub fn squeeze(self, default: V, range: RangeInclusive<T>) -> Anim<T, V, impl Fun<T, V>> {
        Anim::from(move |t| {
            if range.contains(&t) {
                self.eval((t - *range.start()) * T::one() / (*range.end() - *range.start()))
            } else {
                default
            }
        })
    }

    pub fn map<W: Float>(self, f: impl Fn(V) -> W) -> Anim<T, W, impl Fun<T, W>> {
        Anim::from(move |t| f(self.eval(t)))
    }

    pub fn sin(self) -> Anim<T, V, impl Fun<T, V>> {
        self.map(Float::sin)
    }

    pub fn cos(self) -> Anim<T, V, impl Fun<T, V>> {
        self.map(Float::cos)
    }

    pub fn abs(self) -> Anim<T, V, impl Fun<T, V>> {
        self.map(Float::abs)
    }

    pub fn scale_min_max(self, min: V, max: V) -> Anim<T, V, impl Fun<T, V>> {
        self * (max - min) + min
    }
}

pub fn func<T: Float, V: Float>(f: impl Fn(T) -> V) -> Anim<T, V, impl Fun<T, V>> {
    From::from(f)
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

pub fn proportional<T, V>(m: V) -> Anim<T, V, impl Fun<T, V>>
where
    T: Float,
    V: Float + From<T>,
{
    Anim::from(move |t| m * From::from(t))
}

pub fn full_circle<T, V>() -> Anim<T, V, impl Fun<T, V>>
where
    T: Float,
    V: Float + FloatConst + From<T>,
{
    proportional(V::PI() * (V::one() + V::one()))
}

pub fn half_circle<T, V>() -> Anim<T, V, impl Fun<T, V>>
where
    T: Float,
    V: Float + FloatConst + From<T>,
{
    proportional(V::PI())
}

pub fn cond<T, V, F1, F2, A1, A2>(cond: bool, a1: A1, a2: A2) -> Anim<T, V, impl Fun<T, V>>
where
    T: Float,
    V: Float,
    F1: Fun<T, V>,
    F2: Fun<T, V>,
    A1: Into<Anim<T, V, F1>>,
    A2: Into<Anim<T, V, F2>>,
{
    let a1 = a1.into();
    let a2 = a2.into();

    Anim::from(move |t| if cond { a1.eval(t) } else { a2.eval(t) })
}

impl<T, V, F> From<F> for Anim<T, V, WrapFn<T, V, F>>
where
    T: Float,
    V: Float,
    F: Fn(T) -> V,
{
    fn from(f: F) -> Self {
        Anim::new(WrapFn(f, PhantomData))
    }
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

impl<T, V> From<V> for Anim<T, V, ConstantClosure<V>>
where
    T: Float,
    V: Float,
{
    fn from(v: V) -> Self {
        Anim::new(ConstantClosure(v))
    }
}

pub struct ConstantClosure<V>(V);

impl<T, V> Fun<T, V> for ConstantClosure<V>
where
    V: Float,
{
    fn eval(&self, _: T) -> V {
        self.0
    }
}

impl<T, V, F> Add<V> for Anim<T, V, F>
where
    T: Float,
    V: Float,
    F: Fun<T, V>,
{
    type Output = Anim<T, V, AddConstantClosure<T, V, F>>;

    fn add(self, rhs: V) -> Self::Output {
        Anim::new(AddConstantClosure(self, rhs))
    }
}

pub struct AddConstantClosure<T, V, F>(Anim<T, V, F>, V);

impl<T, V, F> Fun<T, V> for AddConstantClosure<T, V, F>
where
    T: Float,
    V: Float,
    F: Fun<T, V>,
{
    fn eval(&self, t: T) -> V {
        self.0.eval(t) + self.1
    }
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
