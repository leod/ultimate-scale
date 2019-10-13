use std::marker::PhantomData;

use glium::uniforms::UniformType;

use crate::render::pipeline::InstanceParams;

pub const V_WORLD_NORMAL: &str = "v_world_normal";
pub const V_WORLD_POS: &str = "v_world_pos";
pub const V_COLOR: &str = "v_color";
pub const V_POSITION: &str = "gl_Position";

pub const F_COLOR: &str = "f_color";

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VariableQualifier {
    Flat,
    Smooth,
}

pub type VariableName = String;
pub type GLSL = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDefs(pub Vec<(VariableName, VariableQualifier, UniformType)>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexCore<P: InstanceParams, V: glium::vertex::Vertex> {
    pub extra_uniforms: VariableDefs,
    pub output_defs: VariableDefs,
    pub defs: GLSL,
    pub body: GLSL,
    pub output_exprs: Vec<(VariableName, GLSL)>,
    pub phantom: PhantomData<(P, V)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentCore<P: InstanceParams> {
    pub extra_uniforms: VariableDefs,
    pub input_defs: VariableDefs,
    pub defs: String,
    pub body: String,
    pub outputs: Vec<(VariableName, UniformType, GLSL)>,
    pub phantom: PhantomData<P>,
}

pub struct Core<P: InstanceParams, V: glium::vertex::Vertex> {
    pub vertex: VertexCore<P, V>,
    pub fragment: FragmentCore<P>,
}

#[macro_export]
macro_rules! shader_output_exprs {
    { $($variable:expr => $expr:literal),*, } => {
        vec![
            $(
                ($variable.to_string(), $expr.to_string()),
            )*
        ]
    }
}

impl<P: InstanceParams, V: glium::vertex::Vertex> Default for VertexCore<P, V> {
    fn default() -> Self {
        Self {
            extra_uniforms: VariableDefs(Vec::new()),
            output_defs: VariableDefs(Vec::new()),
            defs: "".into(),
            body: "".into(),
            output_exprs: Vec::new(),
            phantom: PhantomData,
        }
    }
}

impl<P: InstanceParams> Default for FragmentCore<P> {
    fn default() -> Self {
        Self {
            extra_uniforms: VariableDefs(Vec::new()),
            input_defs: VariableDefs(Vec::new()),
            defs: "".into(),
            body: "".into(),
            outputs: Vec::new(),
            phantom: PhantomData,
        }
    }
}
