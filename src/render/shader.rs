use std::marker::PhantomData;

use glium::uniforms::{UniformType, UniformValue, Uniforms};
use glium::vertex::AttributeType;

use crate::render::pipeline::InstanceParams;

pub const V_WORLD_NORMAL: &str = "v_world_normal";
pub const V_WORLD_POS: &str = "v_world_pos";
pub const V_COLOR: &str = "v_color";
pub const V_POSITION: &str = "gl_Position";

pub const F_COLOR: &str = "f_color";
pub const F_SHADOW: &str = "f_shadow";

pub fn v_world_normal_def() -> SharedVariableDef {
    (
        V_WORLD_NORMAL.into(),
        UniformType::FloatVec3,
        VariableQualifier::Smooth,
    )
}

pub fn v_world_pos_def() -> SharedVariableDef {
    (
        V_WORLD_POS.into(),
        UniformType::FloatVec4,
        VariableQualifier::Smooth,
    )
}

pub fn v_color_def() -> SharedVariableDef {
    (
        V_COLOR.into(),
        UniformType::FloatVec3,
        VariableQualifier::Smooth,
    )
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VariableQualifier {
    Flat,
    Smooth,
    Local,
}

pub type VariableName = String;
pub type GLSL = String;

pub type VariableDef = (VariableName, UniformType);
pub type SharedVariableDef = (VariableName, UniformType, VariableQualifier);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexCore<P: InstanceParams, V: glium::vertex::Vertex> {
    pub extra_uniforms: Vec<VariableDef>,
    pub output_defs: Vec<SharedVariableDef>,
    pub defs: GLSL,
    pub body: GLSL,
    pub output_exprs: Vec<(VariableName, GLSL)>,
    pub phantom: PhantomData<(P, V)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentCore<P: InstanceParams> {
    pub extra_uniforms: Vec<VariableDef>,
    pub input_defs: Vec<SharedVariableDef>,
    pub defs: String,
    pub body: String,
    pub outputs: Vec<(VariableName, UniformType, GLSL)>,
    pub phantom: PhantomData<P>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Core<P: InstanceParams, V: glium::vertex::Vertex> {
    pub vertex: VertexCore<P, V>,
    pub fragment: FragmentCore<P>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedCore<P: InstanceParams, V: glium::vertex::Vertex> {
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
    };
    { $($variable:expr, $type:expr => $expr:literal),*, } => {
        vec![
            $(
                ($variable.to_string(), $type, $expr.to_string()),
            )*
        ]
    };
}

impl<P: InstanceParams, V: glium::vertex::Vertex> Default for VertexCore<P, V> {
    fn default() -> Self {
        Self {
            extra_uniforms: Vec::new(),
            output_defs: Vec::new(),
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
            extra_uniforms: Vec::new(),
            input_defs: Vec::new(),
            defs: "".into(),
            body: "".into(),
            outputs: Vec::new(),
            phantom: PhantomData,
        }
    }
}

impl<P: InstanceParams, V: glium::vertex::Vertex> VertexCore<P, V> {
    pub fn get_output_expr(&self, name: &str) -> Option<&GLSL> {
        self.output_exprs
            .iter()
            .find(|(n, _expr)| n == name)
            .map(|(_n, expr)| expr)
    }

    pub fn with_extra_uniform(mut self, def: VariableDef) -> Self {
        // TODO: Check for duplicates
        self.extra_uniforms.push(def);
        self
    }

    pub fn with_defs(mut self, defs: &str) -> Self {
        self.defs += defs;
        self
    }

    pub fn with_body(mut self, body: &str) -> Self {
        self.body += body;
        self
    }

    pub fn with_output(mut self, def: SharedVariableDef, expr: &str) -> Self {
        assert!(self.get_output_expr(&def.0).is_none());

        self.output_exprs.push((def.0.clone(), expr.into()));
        self.output_defs.push(def);
        self
    }
}

impl<P: InstanceParams> FragmentCore<P> {
    pub fn get_input_def(&self, name: &str) -> Option<&SharedVariableDef> {
        self.input_defs.iter().find(|(n, _t, _q)| n == name)
    }

    pub fn get_output_expr(&self, name: &str) -> Option<&GLSL> {
        self.outputs
            .iter()
            .find(|(n, _t, _expr)| n == name)
            .map(|(_n, _t, expr)| expr)
    }

    pub fn with_extra_uniform(mut self, def: VariableDef) -> Self {
        // TODO: Check for duplicates
        self.extra_uniforms.push(def);
        self
    }

    pub fn with_input_def(mut self, (name, t, q): SharedVariableDef) -> Self {
        match self.get_input_def(&name) {
            Some((_, cur_t, cur_q)) if *cur_t != t => panic!(
                "FragmentCore already has input `{}', but with type {:?} instead of {:?}",
                name, cur_t, t
            ),
            Some((_, cur_t, cur_q)) if *cur_q != q => panic!(
                "FragmentCore already has input `{}', but with qualifier {:?} instead of {:?}",
                name, cur_q, q
            ),
            Some(_) => self,
            None => {
                self.input_defs.push((name, t, q));
                self
            }
        }
    }

    pub fn with_output(mut self, name: &str, t: UniformType, expr: &str) -> Self {
        assert!(self.get_output_expr(name).is_none());

        self.outputs.push((name.into(), t, expr.into()));
        self
    }

    pub fn with_updated_output(mut self, name: &str, f: impl Fn(&str) -> String) -> Self {
        for i in 0..self.outputs.len() {
            if self.outputs[i].0 == name {
                self.outputs[i].2 = f(&self.outputs[i].2);
                return self;
            }
        }

        panic!("FragmentCore does not contain output `{}'", name);
    }

    pub fn with_defs(mut self, defs: &str) -> Self {
        self.defs += defs;
        self
    }

    pub fn with_body(mut self, body: &str) -> Self {
        self.body += body;
        self
    }
}

impl<P: InstanceParams, V: glium::vertex::Vertex> Core<P, V> {
    pub fn link(&self) -> LinkedCore<P, V> {
        // TODO: Remove unused shared variables
        // TODO: Check non-duplicate inputs/outputs
        LinkedCore {
            vertex: self.vertex.clone(),
            fragment: self.fragment.clone(),
        }
    }
}

impl<P: InstanceParams + Default, V: glium::vertex::Vertex> Core<P, V> {
    pub fn build_program<F: glium::backend::Facade>(
        &self,
        facade: &F,
    ) -> Result<glium::Program, glium::program::ProgramCreationError> {
        self.link().build_program(facade)
    }
}

impl<P: InstanceParams + Default, V: glium::vertex::Vertex> LinkedCore<P, V> {
    pub fn build_program<F: glium::backend::Facade>(
        &self,
        facade: &F,
    ) -> Result<glium::Program, glium::program::ProgramCreationError> {
        let vertex = self.vertex.compile();
        let fragment = self.fragment.compile();

        glium::Program::from_source(facade, &vertex, &fragment, None)
    }
}

fn compile_uniform_type(t: UniformType) -> &'static str {
    match t {
        UniformType::Float => "float",
        UniformType::FloatVec2 => "vec2",
        UniformType::FloatVec3 => "vec3",
        UniformType::FloatVec4 => "vec4",
        UniformType::FloatMat2 => "mat2",
        UniformType::FloatMat3 => "mat3",
        UniformType::FloatMat4 => "mat4",
        UniformType::Int => "int",
        UniformType::IntVec2 => "ivec2",
        UniformType::IntVec3 => "ivec3",
        UniformType::IntVec4 => "ivec4",
        UniformType::Sampler2d => "sampler2D",
        _ => unimplemented!("Given UniformType not yet supported: {:?}", t),
    }
}

fn uniform_value_to_type<'a>(v: UniformValue<'a>) -> UniformType {
    match v {
        UniformValue::Float(_) => UniformType::Float,
        UniformValue::Vec2(_) => UniformType::FloatVec2,
        UniformValue::Vec3(_) => UniformType::FloatVec3,
        UniformValue::Vec4(_) => UniformType::FloatVec4,
        UniformValue::Mat2(_) => UniformType::FloatMat2,
        UniformValue::Mat3(_) => UniformType::FloatMat3,
        UniformValue::Mat4(_) => UniformType::FloatMat4,
        UniformValue::SignedInt(_) => UniformType::Int,
        UniformValue::IntVec2(_) => UniformType::IntVec2,
        UniformValue::IntVec3(_) => UniformType::IntVec3,
        UniformValue::IntVec4(_) => UniformType::IntVec4,
        _ => unimplemented!("Given UniformValue not yet supported"),
    }
}

fn compile_variable_def(prefix: &str, (name, t): &VariableDef) -> String {
    prefix.to_string() + " " + &compile_uniform_type(*t).to_string() + " " + name + ";\n"
}

fn compile_variable_defs<'a>(
    prefix: &str,
    defs: impl Iterator<Item = VariableDef>,
) -> String {
    defs.map(|def| compile_variable_def(prefix, &def))
        .collect::<Vec<_>>()
        .join("")
}

fn compile_shared_variable_def(prefix: &str, (name, t, q): &SharedVariableDef) -> String {
    let prefix = match q {
        VariableQualifier::Flat => "flat ".to_string() + prefix,
        VariableQualifier::Smooth => "smooth ".to_string() + prefix,
        VariableQualifier::Local => "".to_string(),
    };

    compile_variable_def(&prefix, &(name.to_string(), *t))
}

fn compile_shared_variable_defs(prefix: &str, defs: &[SharedVariableDef]) -> String {
    defs.iter()
        .map(|def| compile_shared_variable_def(prefix, def))
        .collect::<Vec<_>>()
        .join("")
}

fn compile_instance_params_uniforms<P: InstanceParams + Default>() -> String {
    let mut uniforms = Vec::new();

    P::default().uniforms().visit_values(|name, uniform_value| {
        uniforms.push((name.to_string(), uniform_value_to_type(uniform_value)));
    });

    compile_variable_defs("uniform", uniforms.iter().cloned())
}

fn compile_output_assignment((name, expr): (VariableName, GLSL)) -> String {
    "    ".to_string() + &name + " = " + &expr + ";\n"
}

fn compile_output_assignments(exprs: impl Iterator<Item = (VariableName, GLSL)>) -> String {
    exprs
        .map(compile_output_assignment)
        .collect::<Vec<_>>()
        .join("")
}

fn attribute_type_to_uniform_type(t: AttributeType) -> UniformType {
    match t {
        AttributeType::F32 => UniformType::Float,
        AttributeType::F32F32 => UniformType::FloatVec2,
        AttributeType::F32F32F32 => UniformType::FloatVec3,
        AttributeType::F32F32F32F32 => UniformType::FloatVec4,
        _ => unimplemented!("Given AttributeType not yet supported: {:?}", t),
    }
}

fn compile_vertex_attributes<V: glium::vertex::Vertex>() -> String {
    let bindings = V::build_bindings();

    let mut attributes = Vec::new();
    for i in 0..bindings.len() {
        attributes.push((
            bindings[i].0.to_string(),
            attribute_type_to_uniform_type(bindings[i].2),
        ));
    }

    compile_variable_defs("in", attributes.iter().cloned())
}

impl<P: InstanceParams + Default, V: glium::vertex::Vertex> VertexCore<P, V> {
    pub fn compile(&self) -> String {
        let mut s = String::new();

        s += "#version 330\n\n";

        s += &compile_instance_params_uniforms::<P>();
        s += "\n";
        s += &compile_variable_defs("uniform", self.extra_uniforms.iter().cloned());
        s += "\n";
        s += &compile_vertex_attributes::<V>();
        s += "\n";
        s += &compile_shared_variable_defs("out", &self.output_defs);
        s += "\n";

        s += &self.defs;
        s += "\n";

        s += "void main() {\n";
        s += &self.body;
        s += "\n";
        s += &compile_output_assignments(self.output_exprs.iter().cloned());
        s += "}\n";

        s
    }
}

impl<P: InstanceParams + Default> FragmentCore<P> {
    pub fn compile(&self) -> String {
        let mut s = String::new();

        s += "#version 330\n\n";

        s += &compile_instance_params_uniforms::<P>();
        s += "\n";
        s += &compile_variable_defs("uniform", self.extra_uniforms.iter().cloned());
        s += "\n";
        s += &compile_shared_variable_defs("in", &self.input_defs);
        s += "\n";
        s += &compile_variable_defs(
            "out",
            self.outputs
                .iter()
                .map(|(name, t, _expr)| (name.clone(), *t)),
        );
        s += "\n";

        s += &self.defs;
        s += "\n";

        s += "void main() {\n";
        s += &self.body;
        s += "\n";
        s += &compile_output_assignments(
            self.outputs
                .iter()
                .map(|(name, _t, expr)| (name.clone(), expr.clone())),
        );
        s += "}\n";

        s
    }
}
