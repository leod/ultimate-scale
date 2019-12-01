use std::marker::PhantomData;
use std::ops::{Add, Mul, Neg, RangeInclusive, Sub};

use num_traits::{Float, FloatConst, Num, One, Zero};

pub trait Fun {
    type T;
    type V;

    fn eval(&self, t: Self::T) -> Self::V;
}

#[derive(Clone, Debug)]
pub struct Anim<F>(F);

impl<F> Anim<F>
where
    F: Fun,
{
    pub fn map<W>(self, f: impl Fn(F::V) -> W) -> Anim<impl Fun<T = F::T, V = W>> {
        self.map_anim(func(f))
    }

    pub fn map_anim<W, G, A>(self, anim: A) -> Anim<impl Fun<T = F::T, V = W>>
    where
        G: Fun<T = F::V, V = W>,
        A: Into<Anim<G>>,
    {
        let anim = anim.into();
        func(move |t| anim.eval(self.eval(t)))
    }

    pub fn map_time<S>(self, f: impl Fn(S) -> F::T) -> Anim<impl Fun<T = S, V = F::V>> {
        self.map_time_anim(func(f))
    }

    pub fn map_time_anim<S, G, A>(self, anim: A) -> Anim<impl Fun<T = S, V = F::V>>
    where
        G: Fun<T = S, V = F::T>,
        A: Into<Anim<G>>,
    {
        let anim = anim.into();
        func(move |t| self.eval(anim.eval(t)))
    }
}

impl<F> Fun for Anim<F>
where
    F: Fun,
{
    type T = F::T;
    type V = F::V;

    fn eval(&self, t: F::T) -> F::V {
        self.0.eval(t)
    }
}

impl<F> Anim<F>
where
    F: Fun,
    F::T: Copy,
{
    pub fn zip<W, G, A>(self, other: A) -> Anim<impl Fun<T = F::T, V = (F::V, W)>>
    where
        G: Fun<T = F::T, V = W>,
        A: Into<Anim<G>>,
    {
        let other = other.into();

        func(move |t| (self.eval(t), other.eval(t)))
    }

    pub fn bind<W, G>(self, f: impl Fn(F::V) -> Anim<G>) -> Anim<impl Fun<T = F::T, V = W>>
    where
        G: Fun<T = F::T, V = W>,
    {
        func(move |t| {
            f(self.eval(t)).eval(t)
        })
    }
}

impl<F> Anim<F>
where
    F: Fun,
    F::T: Copy + PartialOrd,
{
    pub fn switch<G, A>(self, self_end: F::T, next: A) -> Anim<impl Fun<T = F::T, V = F::V>>
    where
        G: Fun<T = F::T, V = F::V>,
        A: Into<Anim<G>>,
    {
        cond_t(func(move |t| t <= self_end), self, next)
    }
}

impl<F> Anim<F>
where
    F: Fun,
    F::T: Copy + Sub<Output = F::T>,
{
    pub fn shift_time(self, t_add: F::T) -> Anim<impl Fun<T = F::T, V = F::V>> {
        self.map_time(move |t| t - t_add)
    }
}

impl<F> Anim<F>
where
    F: Fun,
    F::T: Copy + PartialOrd + Sub<Output = F::T>,
{
    pub fn seq<G, A>(self, self_end: F::T, next: A) -> Anim<impl Fun<T = F::T, V = F::V>>
    where
        G: Fun<T = F::T, V = F::V>,
        A: Into<Anim<G>>,
    {
        self.switch(self_end, next.into().shift_time(self_end))
    }
}

impl<F> Anim<F>
where
    F: Fun,
    F::T: Copy + Sub<Output = F::T>,
{
    pub fn backwards(self, end: F::T) -> Anim<impl Fun<T = F::T, V = F::V>> {
        func(move |t| self.eval(end - t))
    }
}

impl<F> Anim<F>
where
    F: Fun,
    F::T: Copy,
    F::V: Copy + Num,
{
    pub fn scale_min_max(self, min: F::V, max: F::V) -> Anim<impl Fun<T = F::T, V = F::V>> {
        self * (max - min) + min
    }
}

impl<F> Anim<F>
where
    F: Fun,
    F::V: Float,
{
    pub fn sin(self) -> Anim<impl Fun<T = F::T, V = F::V>> {
        self.map(Float::sin)
    }

    pub fn cos(self) -> Anim<impl Fun<T = F::T, V = F::V>> {
        self.map(Float::cos)
    }

    pub fn abs(self) -> Anim<impl Fun<T = F::T, V = F::V>> {
        self.map(Float::abs)
    }

    pub fn powf(self, e: F::V) -> Anim<impl Fun<T = F::T, V = F::V>> {
        self.map(move |v| v.powf(e))
    }

    pub fn powi(self, n: i32) -> Anim<impl Fun<T = F::T, V = F::V>> {
        self.map(move |v| v.powi(n))
    }
}

impl<F> Anim<F>
where
    F: Fun,
    F::T: Copy + Float,
    F::V: Copy,
{
    pub fn squeeze(
        self,
        default: F::V,
        range: RangeInclusive<F::T>,
    ) -> Anim<impl Fun<T = F::T, V = F::V>> {
        let time_shift = *range.start();
        let time_scale = F::T::one() / (*range.end() - *range.start());

        cond_t(
            move |t| range.contains(&t),
            self.map_time(move |t| (t - time_shift) * time_scale),
            default,
        )
    }
}

impl<W, F> Anim<F>
where
    F: Fun,
    F::T: Copy + Mul<W, Output = W>,
    F::V: Copy + Add<W, Output = F::V> + Sub<Output = W>,
{
    pub fn lerp<G, A>(self, other: A) -> Anim<impl Fun<T = F::T, V = F::V>>
    where
        G: Fun<T = F::T, V = F::V>,
        A: Into<Anim<G>>,
    {
        let other = other.into();

        func(move |t| {
            let a = self.eval(t);
            let b = other.eval(t);

            let delta = b - a;

            a + t * delta
        })
    }
}

impl<V, F> Anim<F>
where
    F: Fun<V = Option<V>>,
    F::T: Copy,
{
    pub fn unwrap_or<G, A>(
        self,
        default: A,
    ) -> Anim<impl Fun<T = F::T, V = V>>
    where
        G: Fun<T = F::T, V = V>,
        A: Into<Anim<G>>,
    {
        self.zip(default.into()).map(|(v, default)| v.unwrap_or(default))
    }

    pub fn map_or<W, G, H, A>(
        self,
        default: A,
        f: impl Fn(V) -> Anim<H>,
    ) -> Anim<impl Fun<T = F::T, V = W>>
    where
        G: Fun<T = F::T, V = W>,
        H: Fun<T = F::T, V = W>,
        A: Into<Anim<G>>,
    {
        let default = default.into();

        //self.bind(move |v| v.map_or(default, f))

        func(move |t| {
            self.eval(t)
                .map_or_else(|| default.eval(t), |v| f(v).eval(t))
        })
    }
}

pub fn func<T, V>(f: impl Fn(T) -> V) -> Anim<impl Fun<T = T, V = V>> {
    From::from(f)
}

pub fn constant<T, V: Copy>(c: V) -> Anim<impl Fun<T = T, V = V>> {
    func(move |_| c)
}

pub fn one<T, V: Copy + One>() -> Anim<impl Fun<T = T, V = V>> {
    constant(V::one())
}

pub fn zero<T, V: Copy + Zero>() -> Anim<impl Fun<T = T, V = V>> {
    constant(V::zero())
}

pub fn proportional<T, V>(m: V) -> Anim<impl Fun<T = T, V = V>>
where
    T: Float,
    V: Float + From<T>,
{
    func(move |t| m * From::from(t))
}

pub fn full_circle<T, V>() -> Anim<impl Fun<T = T, V = V>>
where
    T: Float,
    V: Float + FloatConst + From<T>,
{
    proportional(V::PI() * (V::one() + V::one()))
}

pub fn half_circle<T, V>() -> Anim<impl Fun<T = T, V = V>>
where
    T: Float,
    V: Float + FloatConst + From<T>,
{
    proportional(V::PI())
}

pub fn quarter_circle<T, V>() -> Anim<impl Fun<T = T, V = V>>
where
    T: Float,
    V: Float + FloatConst + From<T>,
{
    proportional(V::PI() * (V::one() / (V::one() + V::one())))
}

pub fn cond_t<T, V, F, G, H, Cond, A, B>(cond: Cond, a: A, b: B) -> Anim<impl Fun<T = T, V = V>>
where
    T: Copy,
    F: Fun<T = T, V = bool>,
    G: Fun<T = T, V = V>,
    H: Fun<T = T, V = V>,
    Cond: Into<Anim<F>>,
    A: Into<Anim<G>>,
    B: Into<Anim<H>>,
{
    let cond = cond.into();
    let a = a.into();
    let b = b.into();

    func(move |t| if cond.eval(t) { a.eval(t) } else { b.eval(t) })
}

pub fn cond<T, V, F, G, A, B>(cond: bool, a: A, b: B) -> Anim<impl Fun<T = T, V = V>>
where
    T: Copy,
    F: Fun<T = T, V = V>,
    G: Fun<T = T, V = V>,
    A: Into<Anim<F>>,
    B: Into<Anim<G>>,
{
    cond_t(func(move |_| cond), a, b)
}

pub fn lerp<T, V, W, F, G, A, B>(a: A, b: B) -> Anim<impl Fun<T = T, V = V>>
where
    T: Copy + Mul<W, Output = W>,
    V: Copy + Add<W, Output = V> + Sub<Output = W>,
    F: Fun<T = T, V = V>,
    G: Fun<T = T, V = V>,
    A: Into<Anim<F>>,
    B: Into<Anim<G>>,
{
    a.into().lerp(b.into())
}

pub fn cubic<T>(w: &[T; 4]) -> Anim<impl Fun<T = T, V = T> + '_>
where
    T: Float,
{
    func(move |t| {
        let t2 = t * t;
        let t3 = t2 * t;

        w[0] * t3 + w[1] * t2 + w[2] * t + w[3]
    })
}

#[macro_export]
macro_rules! anim_match {
    (
        $expr:expr;
        $($pat:pat => $value:expr $(,)?)*
    ) => {
        $crate::util::anim::func(move |t| match $expr {
            $(
                $pat => ($crate::util::anim::Anim::from($value)).eval(t),
            )*
        })
    }
}

impl<T, V, F> From<F> for Anim<WrapFn<T, V, F>>
where
    F: Fn(T) -> V,
{
    fn from(f: F) -> Self {
        Anim(WrapFn(f, PhantomData))
    }
}

struct WrapFn<T, V, F: Fn(T) -> V>(F, PhantomData<(T, V)>);

impl<T, V, F> Fun for WrapFn<T, V, F>
where
    F: Fn(T) -> V,
{
    type T = T;
    type V = V;

    fn eval(&self, t: T) -> V {
        self.0(t)
    }
}

impl<F, G> Add<Anim<G>> for Anim<F>
where
    F: Fun,
    G: Fun<T = F::T>,
    F::V: Add<G::V>,
{
    type Output = Anim<AddClosure<F, G>>;

    fn add(self, rhs: Anim<G>) -> Self::Output {
        Anim(AddClosure(self.0, rhs.0))
    }
}

impl<V, F> Add<V> for Anim<F>
where
    V: Copy,
    F: Fun<V = V>,
{
    type Output = Anim<AddClosure<F, ConstantClosure<F::T, F::V>>>;

    fn add(self, rhs: F::V) -> Self::Output {
        Anim(AddClosure(self.0, ConstantClosure::from(rhs)))
    }
}

impl<F, G> Sub<Anim<G>> for Anim<F>
where
    F: Fun,
    G: Fun<T = F::T>,
    F::V: Sub<G::V>,
{
    type Output = Anim<AddClosure<F, NegClosure<G>>>;

    fn sub(self, rhs: Anim<G>) -> Self::Output {
        Anim(AddClosure(self.0, NegClosure(rhs.0)))
    }
}

impl<F, G> Mul<Anim<G>> for Anim<F>
where
    F: Fun,
    F::T: Copy,
    G: Fun<T = F::T>,
    F::V: Mul<G::V>,
{
    type Output = Anim<MulClosure<F, G>>;

    fn mul(self, rhs: Anim<G>) -> Self::Output {
        Anim(MulClosure(self.0, rhs.0))
    }
}

impl<V, F> Mul<V> for Anim<F>
where
    V: Copy,
    F: Fun<V = V>,
    F::T: Copy,
{
    type Output = Anim<MulClosure<F, ConstantClosure<F::T, F::V>>>;

    fn mul(self, rhs: F::V) -> Self::Output {
        Anim(MulClosure(self.0, ConstantClosure::from(rhs)))
    }
}

impl<V, F> Neg for Anim<F>
where
    V: Copy,
    F: Fun<V = V>,
{
    type Output = Anim<NegClosure<F>>;

    fn neg(self) -> Self::Output {
        Anim(NegClosure(self.0))
    }
}

pub struct ConstantClosure<T, V>(V, PhantomData<T>);

impl<T, V> Fun for ConstantClosure<T, V>
where
    T: Copy,
    V: Copy,
{
    type T = T;
    type V = V;

    fn eval(&self, _: T) -> V {
        self.0
    }
}

impl<T, V> From<V> for ConstantClosure<T, V>
where
    V: Copy,
{
    fn from(v: V) -> Self {
        ConstantClosure(v, PhantomData)
    }
}

impl<T, V> From<V> for Anim<ConstantClosure<T, V>>
where
    V: Copy,
{
    fn from(v: V) -> Self {
        Anim(ConstantClosure::from(v))
    }
}

pub struct AddClosure<F, G>(F, G);

impl<F, G> Fun for AddClosure<F, G>
where
    F: Fun,
    F::T: Copy,
    G: Fun<T = F::T>,
    F::V: Add<G::V>,
{
    type T = F::T;
    type V = <F::V as Add<G::V>>::Output;

    fn eval(&self, t: F::T) -> Self::V {
        self.0.eval(t) + self.1.eval(t)
    }
}

pub struct MulClosure<F, G>(F, G);

impl<F, G> Fun for MulClosure<F, G>
where
    F: Fun,
    F::T: Copy,
    G: Fun<T = F::T>,
    F::V: Mul<G::V>,
{
    type T = F::T;
    type V = <F::V as Mul<G::V>>::Output;

    fn eval(&self, t: F::T) -> Self::V {
        self.0.eval(t) * self.1.eval(t)
    }
}

pub struct NegClosure<F>(F);

impl<F> Fun for NegClosure<F>
where
    F: Fun,
    F::V: Neg,
{
    type T = F::T;
    type V = <F::V as Neg>::Output;

    fn eval(&self, t: F::T) -> Self::V {
        -self.0.eval(t)
    }
}
