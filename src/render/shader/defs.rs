use glium::uniforms::UniformType;

use crate::render::shader::{VertexOutQualifier, FragmentOutQualifier, VertexOutDef, FragmentOutDef};

pub const V_WORLD_NORMAL: &str = "v_world_normal";
pub const V_WORLD_POS: &str = "v_world_pos";
pub const V_COLOR: &str = "v_color";
pub const V_TEX_COORD: &str = "v_tex_coord";
pub const V_POSITION: &str = "gl_Position";

pub const F_COLOR: &str = "f_color";
pub const F_FRAGMENT_DEPTH: &str = "f_fragment_depth";
pub const F_SHADOW: &str = "f_shadow";

pub fn v_world_normal() -> VertexOutDef {
    (
        (V_WORLD_NORMAL.into(), UniformType::FloatVec3),
        VertexOutQualifier::Smooth,
    )
}

pub fn v_world_pos() -> VertexOutDef {
    (
        (V_WORLD_POS.into(), UniformType::FloatVec4),
        VertexOutQualifier::Smooth,
    )
}

pub fn v_color() -> VertexOutDef {
    (
        (V_COLOR.into(), UniformType::FloatVec3),
        VertexOutQualifier::Smooth,
    )
}

pub fn v_tex_coord() -> VertexOutDef {
    (
        (V_TEX_COORD.into(), UniformType::FloatVec2),
        VertexOutQualifier::Smooth,
    )
}

pub fn f_color() -> FragmentOutDef {
    (
        (F_COLOR.into(), UniformType::FloatVec4),
        FragmentOutQualifier::Yield,
    )
}

pub fn f_fragment_depth() -> FragmentOutDef {
    (
        (F_FRAGMENT_DEPTH.into(), UniformType::Float),
        FragmentOutQualifier::Yield,
    )
}

pub fn f_shadow() -> FragmentOutDef {
    (
        (F_SHADOW.into(), UniformType::Float),
        FragmentOutQualifier::Local,
    )
}

