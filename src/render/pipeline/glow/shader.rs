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
pub fn composition_core() -> shader::Core<(), screen_quad::Vertex> {
    let vertex = shader::VertexCore {
        out_defs: vec![shader::v_tex_coord_def()],
        out_exprs: shader_out_exprs! {
            shader::V_TEX_COORD => "tex_coord",
            shader::V_POSITION => "position",
        },
        ..Default::default()
    };

    let fragment = shader::FragmentCore {
        extra_uniforms: vec![
            ("scene_texture".into(), UniformType::Sampler2d),
            ("glow_texture".into(), UniformType::Sampler2d),
        ],
        in_defs: vec![shader::v_tex_coord_def()],
        out_defs: vec![shader::f_color_def()],
        body: "
            vec3 scene_value = texture(scene_texture, v_tex_coord).rgb;
            vec3 glow_value = texture(glow_texture, v_tex_coord).rgb;
        "
        .into(),
        out_exprs: shader_out_exprs! {
            shader::F_COLOR => "vec4(scene_value + glow_value, 1.0)",
        },
        ..Default::default()
    };

    shader::Core { vertex, fragment }
}
