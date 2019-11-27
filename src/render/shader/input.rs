use glium::uniforms::{AsUniformValue, EmptyUniforms, UniformValue, Uniforms, UniformsStorage};

pub struct ToUniformsWrapper<'b, U: ?Sized>(&'b U);

impl<'b, U: ToUniforms> Uniforms for ToUniformsWrapper<'b, U> {
    fn visit_values<'a, F>(&'a self, output: F)
    where
        F: FnMut(&str, UniformValue<'a>),
    {
        ToUniforms::visit_values(self.0, output);
    }
}

pub trait ToUniforms {
    fn visit_values<'a, F>(&'a self, output: F)
    where
        F: FnMut(&str, UniformValue<'a>);

    fn to_uniforms(&self) -> ToUniformsWrapper<'_, Self> {
        ToUniformsWrapper(self)
    }
}

#[macro_export]
macro_rules! to_uniforms_impl {
    ($ty:ident , $this:ident => { $( $field:ident : $type:ident => $value:expr, )* } $(,)? ) => {
        impl $crate::render::shader::ToUniforms for $ty {
            fn visit_values<'a, F>(&'a $this, mut output: F)
            where
                F: FnMut(&str, glium::uniforms::UniformValue<'a>),
            {
                $(
                    output(stringify!($field), glium::uniforms::UniformValue::$type($value));
                )*
            }
        }
    }
}

impl ToUniforms for () {
    fn visit_values<'a, F>(&'a self, _: F)
    where
        F: FnMut(&str, UniformValue<'a>),
    {
    }
}

impl<U1, U2> ToUniforms for (U1, U2)
where
    U1: ToUniforms,
    U2: ToUniforms,
{
    fn visit_values<'a, F>(&'a self, mut output: F)
    where
        F: FnMut(&str, UniformValue<'a>),
    {
        // F is not Copy, so we have to wrap into a lambda here
        self.0.visit_values(|x, y| output(x, y));
        self.1.visit_values(|x, y| output(x, y));
    }
}

impl<U1, U2, U3> ToUniforms for (U1, U2, U3)
where
    U1: ToUniforms,
    U2: ToUniforms,
    U3: ToUniforms,
{
    fn visit_values<'a, F>(&'a self, mut output: F)
    where
        F: FnMut(&str, UniformValue<'a>),
    {
        // F is not Copy, so we have to wrap into a lambda here
        self.0.visit_values(|x, y| output(x, y));
        self.1.visit_values(|x, y| output(x, y));
        self.2.visit_values(|x, y| output(x, y));
    }
}

impl<'b, U> ToUniforms for &'b U
where
    U: ToUniforms,
{
    fn visit_values<'a, F>(&'a self, output: F)
    where
        F: FnMut(&str, UniformValue<'a>),
    {
        U::visit_values(self, output)
    }
}

impl<U> ToUniforms for Option<U>
where
    U: ToUniforms,
{
    fn visit_values<'a, F>(&'a self, output: F)
    where
        F: FnMut(&str, UniformValue<'a>),
    {
        if let Some(uniforms) = self.as_ref() {
            uniforms.visit_values(output);
        }
    }
}

impl ToUniforms for EmptyUniforms {
    fn visit_values<'a, F>(&'a self, _output: F)
    where
        F: FnMut(&str, UniformValue<'a>),
    {
    }
}

impl<'n, T: AsUniformValue, R: Uniforms> ToUniforms for UniformsStorage<'n, T, R> {
    fn visit_values<'a, F>(&'a self, output: F)
    where
        F: FnMut(&str, UniformValue<'a>),
    {
        Uniforms::visit_values(self, output);
    }
}
