//! Glow effect.
//!
//! Heavily inspired by:
//! https://learnopengl.com/Advanced-Lighting/Bloom

use glium::uniforms::UniformType;

use crate::render::pipeline::InstanceParams;
use crate::render::{screen_quad, shader};

pub const F_GLOW_COLOR: &str = "f_glow_color";

pub fn f_glow_color() -> shader::FragmentOutDef {
    (
        (F_GLOW_COLOR.into(), UniformType::FloatVec3),
        shader::FragmentOutQualifier::Yield,
    )
}

/// Shader core transform for rendering color into a texture so that it can be
/// blurred and composed for a glow effect later in the pipeline.
pub fn glow_map_core_transform<P: InstanceParams, V: glium::vertex::Vertex>(
    core: shader::Core<P, V>,
) -> shader::Core<P, V> {
    let fragment = core.fragment.with_out(f_glow_color(), "vec3(f_color)");

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}

/// Shader core for composing the glow texture with the scene texture.
pub fn composition_core_transform(
    core: shader::Core<(), screen_quad::Vertex>,
) -> shader::Core<(), screen_quad::Vertex> {
    assert!(
        core.fragment.has_in(shader::V_TEX_COORD),
        "FragmentCore needs V_TEX_COORD input for glow composition pass"
    );
    assert!(
        core.fragment.has_out(shader::F_COLOR),
        "FragmentCore needs F_COLOR output for glow composition pass"
    );

    let fragment = core
        .fragment
        .with_extra_uniform(("glow_texture".into(), UniformType::Sampler2d))
        .with_out_expr(
            shader::F_COLOR,
            "f_color + vec4(texture(glow_texture, v_tex_coord).rgb, 0.0)",
        );

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}
