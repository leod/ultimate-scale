#[macro_use]
pub mod input;
pub mod defs;

use log::info;

use std::marker::PhantomData;

use glsl::parser::Parse;
use glsl::visitor::Host;

use glium::uniforms::UniformType;
use glium::vertex::AttributeType;

pub use input::{ToUniforms, ToVertex, UniformInput};

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VertexOutQualifier {
    Flat,
    Smooth,
    Local,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FragmentOutQualifier {
    Local,
    Yield,
}

pub type VariableName = String;
pub type GLSL = String;

pub type VariableDef = (VariableName, UniformType);
pub type VertexOutDef = (VariableDef, VertexOutQualifier);
pub type FragmentOutDef = (VariableDef, FragmentOutQualifier);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VertexCore<P, I, V> {
    pub extra_uniforms: Vec<VariableDef>,
    pub out_defs: Vec<VertexOutDef>,
    pub defs: GLSL,
    pub body: GLSL,
    pub out_exprs: Vec<(VariableName, GLSL)>,
    pub phantom: PhantomData<(P, I, V)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentCore<P> {
    pub extra_uniforms: Vec<VariableDef>,
    pub in_defs: Vec<VertexOutDef>,
    pub out_defs: Vec<FragmentOutDef>,
    pub defs: String,
    pub body: String,
    pub out_exprs: Vec<(VariableName, GLSL)>,
    pub phantom: PhantomData<P>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Core<P, I, V> {
    pub vertex: VertexCore<P, I, V>,
    pub fragment: FragmentCore<P>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedCore<P, I, V> {
    pub vertex: VertexCore<P, I, V>,
    pub fragment: FragmentCore<P>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstancingMode {
    Uniforms,
    Vertex,
}

impl<P, I, V> Default for VertexCore<P, I, V> {
    fn default() -> Self {
        Self {
            extra_uniforms: Vec::new(),
            out_defs: Vec::new(),
            defs: "".into(),
            body: "".into(),
            out_exprs: Vec::new(),
            phantom: PhantomData,
        }
    }
}

impl<P> Default for FragmentCore<P> {
    fn default() -> Self {
        Self {
            extra_uniforms: Vec::new(),
            in_defs: Vec::new(),
            out_defs: Vec::new(),
            defs: "".into(),
            body: "".into(),
            out_exprs: Vec::new(),
            phantom: PhantomData,
        }
    }
}

impl<P, I, V> VertexCore<P, I, V> {
    pub fn empty() -> Self {
        Default::default()
    }

    pub fn has_out(&self, name: &str) -> bool {
        self.out_defs
            .iter()
            .filter(|((n, _t), _q)| n == name)
            .count()
            > 0
    }

    pub fn get_out_exprs<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a GLSL> {
        self.out_exprs
            .iter()
            .filter(move |(n, _expr)| n == name)
            .map(|(_n, expr)| expr)
    }

    pub fn with_extra_uniform(mut self, name: &str, t: UniformType) -> Self {
        // TODO: Check for duplicates
        self.extra_uniforms.push((name.into(), t));
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

    pub fn with_out(mut self, def: VertexOutDef, expr: &str) -> Self {
        assert!(self.get_out_exprs(&(def.0).0).count() == 0);

        self.out_exprs.push(((def.0).0.clone(), expr.into()));
        self.out_defs.push(def);
        self
    }

    pub fn with_out_expr(mut self, name: &str, expr: &str) -> Self {
        self.out_exprs.push((name.into(), expr.into()));
        self
    }

    pub fn with_out_def(mut self, def: VertexOutDef) -> Self {
        self.out_defs.push(def);
        self
    }
}

impl<P> FragmentCore<P> {
    pub fn empty() -> Self {
        Default::default()
    }

    pub fn get_in_def(&self, name: &str) -> Option<&VertexOutDef> {
        self.in_defs.iter().find(|((n, _t), _q)| n == name)
    }

    pub fn has_in(&self, name: &str) -> bool {
        self.get_in_def(name).is_some()
    }

    pub fn has_out(&self, name: &str) -> bool {
        self.out_defs
            .iter()
            .filter(|((n, _t), _q)| n == name)
            .count()
            > 0
    }

    pub fn get_out_exprs<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a GLSL> {
        self.out_exprs
            .iter()
            .filter(move |(n, _expr)| n == name)
            .map(|(_n, expr)| expr)
    }

    pub fn with_extra_uniform(mut self, name: &str, t: UniformType) -> Self {
        // TODO: Check for duplicates
        self.extra_uniforms.push((name.into(), t));
        self
    }

    pub fn with_in_def(mut self, ((name, t), q): VertexOutDef) -> Self {
        match self.get_in_def(&name) {
            Some(((_, cur_t), _cur_q)) if *cur_t != t => panic!(
                "FragmentCore already has input `{}', but with type {:?} instead of {:?}",
                name, cur_t, t
            ),
            Some(((_, _cur_t), cur_q)) if *cur_q != q => panic!(
                "FragmentCore already has input `{}', but with qualifier {:?} instead of {:?}",
                name, cur_q, q
            ),
            Some(_) => self,
            None => {
                self.in_defs.push(((name, t), q));
                self
            }
        }
    }

    pub fn with_out(mut self, def: FragmentOutDef, expr: &str) -> Self {
        assert!(self.get_out_exprs(&(def.0).0).count() == 0);

        self.out_exprs.push(((def.0).0.clone(), expr.into()));
        self.out_defs.push(def);
        self
    }

    pub fn with_out_expr(mut self, name: &str, expr: &str) -> Self {
        self.out_exprs.push((name.into(), expr.into()));
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
}

fn does_core_use_variable(
    defs: &str,
    body: &str,
    out_exprs: &[(String, GLSL)],
    var_name: &str,
) -> bool {
    struct Visitor<'a> {
        var_name: &'a str,
        is_used: bool,
    }

    impl<'a> glsl::visitor::Visitor for Visitor<'a> {
        fn visit_identifier(
            &mut self,
            identifier: &mut glsl::syntax::Identifier,
        ) -> glsl::visitor::Visit {
            if identifier.as_str() == self.var_name {
                self.is_used = true;
            }

            glsl::visitor::Visit::Children
        }
    }

    let mut visitor = Visitor {
        var_name,
        is_used: false,
    };

    for (out_name, out_expr) in out_exprs {
        if out_name != var_name {
            glsl::syntax::Expr::parse(out_expr)
                .unwrap()
                .visit(&mut visitor);
        }
    }

    if !body.is_empty() {
        // Putting braces around allows parsing compound statements.
        let body = "{".to_owned() + body + "}";

        let mut body = glsl::syntax::Statement::parse(body).unwrap();
        body.visit(&mut visitor);
    }

    if !defs.is_empty() {
        let mut defs = glsl::syntax::TranslationUnit::parse(defs).unwrap();
        defs.visit(&mut visitor);
    }

    visitor.is_used
}

impl<P, I, V> Core<P, I, V>
where
    P: UniformInput + Clone,
    I: UniformInput + Clone,
    V: glium::vertex::Vertex,
{
    pub fn link(&self) -> LinkedCore<P, I, V> {
        let mut fragment = self.fragment.clone();

        // Remove unused inputs from fragment shader.
        fragment.in_defs = fragment
            .in_defs
            .iter()
            .filter(|((in_name, _), _q)| {
                let r = does_core_use_variable(
                    &fragment.defs,
                    &fragment.body,
                    &fragment.out_exprs,
                    &in_name,
                );

                if !r {
                    info!("Removing fragment input {}", in_name);
                }

                r
            })
            .cloned()
            .collect();

        // Demote vertex shader outputs to local when not needed by fragment
        // shader.
        let mut vertex = self.vertex.clone();

        for ((out_name, _), q) in vertex.out_defs.iter_mut() {
            if !fragment.has_in(&out_name) {
                info!("Demoting vertex output {} to local", out_name);

                *q = VertexOutQualifier::Local;
            }
        }

        // Remove unused local vertex shader outputs.
        //
        // We take the transitive closure by looping, since removing one output
        // may cause another output to become unused.
        let mut changed = true;

        while changed {
            changed = false;

            for ((out_name, _), q) in vertex.out_defs.clone().iter() {
                if *q == VertexOutQualifier::Local {
                    let is_used = does_core_use_variable(
                        &vertex.defs,
                        &vertex.body,
                        &vertex.out_exprs,
                        &out_name,
                    );

                    if !is_used {
                        info!("Removing vertex output {}", out_name);

                        vertex.out_defs.retain(|((name, _), _)| name != out_name);
                        vertex.out_exprs.retain(|(name, _)| name != out_name);

                        changed = true;
                    }
                }
            }
        }

        LinkedCore { vertex, fragment }
    }
}

impl<P, I, V> Core<P, I, V>
where
    P: UniformInput + Clone,
    I: UniformInput + Clone,
    V: glium::vertex::Vertex,
{
    pub fn build_program<F: glium::backend::Facade>(
        &self,
        facade: &F,
        mode: InstancingMode,
    ) -> Result<glium::Program, glium::program::ProgramCreationError> {
        self.link().build_program(facade, mode)
    }
}

impl<P, I, V> LinkedCore<P, I, V>
where
    P: UniformInput,
    I: UniformInput,
    V: glium::vertex::Vertex,
{
    pub fn build_program<F: glium::backend::Facade>(
        &self,
        facade: &F,
        mode: InstancingMode,
    ) -> Result<glium::Program, glium::program::ProgramCreationError> {
        let vertex = self.vertex.compile(mode);
        let fragment = self.fragment.compile();

        // We use the long form of `glium::Program` construction here, since
        // glium by default sets `outputs_rgb` to false, which causes it to
        // enable `GL_FRAMEBUFFER_SRGB` later on when rendering. This
        // apparently has the effect of OpenGL applying gamma correction when
        // rendering to the screen, at least from what I could tell on Ubuntu.
        // Thus, everything turns out too light when already using corrected
        // colors. Seems weird, and there's definitely still something else
        // going on here.
        //
        // Related issue: https://github.com/rust-windowing/glutin/issues/1175
        glium::Program::new(
            facade,
            glium::program::ProgramCreationInput::SourceCode {
                vertex_shader: &vertex,
                fragment_shader: &fragment,
                geometry_shader: None,
                tessellation_control_shader: None,
                tessellation_evaluation_shader: None,
                transform_feedback_varyings: None,
                outputs_srgb: true,
                uses_point_size: false,
            },
        )
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
        UniformType::Bool => "bool",
        _ => unimplemented!("Given UniformType not yet supported: {:?}", t),
    }
}

fn compile_variable_def(prefix: &str, (name, t): &VariableDef) -> String {
    prefix.to_string() + " " + &compile_uniform_type(*t).to_string() + " " + name + ";\n"
}

fn compile_variable_defs(prefix: &str, defs: impl Iterator<Item = VariableDef>) -> String {
    defs.map(|def| compile_variable_def(prefix, &def))
        .collect::<Vec<_>>()
        .join("")
}

fn compile_vertex_out_def(non_local_prefix: &str, ((name, t), q): &VertexOutDef) -> String {
    let prefix = match q {
        VertexOutQualifier::Flat => "flat ".to_string() + non_local_prefix,
        VertexOutQualifier::Smooth => "smooth ".to_string() + non_local_prefix,
        VertexOutQualifier::Local => "".to_string(),
    };

    compile_variable_def(&prefix, &(name.to_string(), *t))
}

fn compile_vertex_out_defs(prefix: &str, defs: &[VertexOutDef]) -> String {
    defs.iter()
        .map(|def| compile_vertex_out_def(prefix, def))
        .collect::<Vec<_>>()
        .join("")
}

fn compile_fragment_out_def(((name, t), q): &FragmentOutDef) -> String {
    let prefix = match q {
        FragmentOutQualifier::Local => "".to_string(),
        FragmentOutQualifier::Yield => "out ".to_string(),
    };

    compile_variable_def(&prefix, &(name.to_string(), *t))
}

fn compile_fragment_out_defs(defs: &[FragmentOutDef]) -> String {
    defs.iter()
        .map(compile_fragment_out_def)
        .collect::<Vec<_>>()
        .join("")
}

fn compile_uniforms<P: UniformInput>() -> String {
    let uniforms = P::uniform_input_defs();

    compile_variable_defs("uniform", uniforms.iter().cloned())
}

fn compile_instance_input<P: UniformInput>(mode: InstancingMode) -> String {
    let uniforms = P::uniform_input_defs();

    match mode {
        InstancingMode::Uniforms => compile_variable_defs("uniform", uniforms.iter().cloned()),
        InstancingMode::Vertex => compile_variable_defs("in", uniforms.iter().cloned()),
    }
}

fn compile_out_assignment((name, expr): (VariableName, GLSL)) -> String {
    "    ".to_string() + &name + " = " + &expr + ";\n"
}

fn compile_out_assignments(exprs: impl Iterator<Item = (VariableName, GLSL)>) -> String {
    exprs
        .map(compile_out_assignment)
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

impl<P, I, V> VertexCore<P, I, V>
where
    P: UniformInput,
    I: UniformInput,
    V: glium::vertex::Vertex,
{
    pub fn compile(&self, mode: InstancingMode) -> String {
        let mut s = String::new();

        s += "#version 330\n\n";

        s += &compile_uniforms::<P>();
        s += "\n";
        s += &compile_instance_input::<I>(mode);
        s += "\n";
        s += &compile_variable_defs("uniform", self.extra_uniforms.iter().cloned());
        s += "\n";
        s += &compile_vertex_attributes::<V>();
        s += "\n";
        s += &compile_vertex_out_defs("out", &self.out_defs);
        s += "\n";

        s += &self.defs;
        s += "\n";

        s += "void main() {\n";
        s += &self.body;
        s += "\n";
        s += &compile_out_assignments(self.out_exprs.iter().cloned());
        s += "}\n";

        s
    }
}

impl<P> FragmentCore<P>
where
    P: UniformInput,
{
    pub fn compile(&self) -> String {
        let mut s = String::new();

        s += "#version 330\n\n";

        s += &compile_uniforms::<P>();
        s += "\n";
        s += &compile_variable_defs("uniform", self.extra_uniforms.iter().cloned());
        s += "\n";
        s += &compile_vertex_out_defs("in", &self.in_defs);
        s += "\n";
        s += &compile_fragment_out_defs(&self.out_defs);
        s += "\n";

        s += &self.defs;
        s += "\n";

        s += "void main() {\n";
        s += &self.body;
        s += "\n";
        s += &compile_out_assignments(self.out_exprs.iter().cloned());
        s += "}\n";

        s
    }
}
