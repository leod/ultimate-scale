use glium::uniforms::UniformType;

use crate::render::pipeline::{Context, InstanceParams};
use crate::render::shader;

pub const F_GLOW_COLOR: &str = "f_glow_color";

pub fn f_glow_color() -> shader::FragmentOutDef {
    (
        (F_GLOW_COLOR.into(), UniformType::FloatVec3),
        shader::FragmentOutQualifier::Yield,
    )
}

/// Shader core for rendering color into a texture so that it can be blurred
/// and composed for a glow effect later in the pipeline.
pub fn glow_map_core_transform<P: InstanceParams, V: glium::vertex::Vertex>(
    core: shader::Core<P, V>,
) -> shader::Core<P, V> {
    let fragment = core.fragment.with_out(f_glow_color(), "f_color");

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}
