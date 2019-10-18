use glium::uniforms::UniformType;

use crate::render::pipeline::InstanceParams;
use crate::render::shader;

pub const F_WORLD_POS: &str = "f_world_pos";
pub const F_WORLD_NORMAL: &str = "f_world_normal";

pub fn f_world_pos_def() -> shader::FragmentOutDef {
    (
        (F_WORLD_POS.into(), UniformType::FloatVec4),
        shader::FragmentOutQualifier::Yield,
    )
}

pub fn f_world_normal_def() -> shader::FragmentOutDef {
    (
        (F_WORLD_NORMAL.into(), UniformType::FloatVec4),
        shader::FragmentOutQualifier::Yield,
    )
}

/// Shader core for writing position/normal/color into separate buffers, so
/// that they may be combined in a subsequent pass.
pub fn scene_buffers_core_transform<P: InstanceParams, V: glium::vertex::Vertex>(
    core: shader::Core<P, V>,
) -> shader::Core<P, V> {
    assert!(
        core.vertex.has_out(shader::V_WORLD_POS),
        "VertexCore needs V_WORLD_POS output for deferred shading"
    );
    assert!(
        core.vertex.has_out(shader::V_WORLD_NORMAL),
        "VertexCore needs V_WORLD_NORMAL output for deferred shading"
    );
    assert!(
        core.fragment.has_out(shader::F_COLOR),
        "FragmentCore needs F_COLOR output for deferred shading"
    );

    let fragment = core
        .fragment
        .with_in_def(shader::v_world_pos_def())
        .with_in_def(shader::v_world_normal_def())
        .with_out(f_world_pos_def(), "v_world_pos")
        .with_out(f_world_normal_def(), "vec4(v_world_normal, 0.0)");

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}
