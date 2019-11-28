//! Glow effect.
//!
//! Heavily inspired by:
//! https://learnopengl.com/Advanced-Lighting/Bloom

use glium::uniforms::UniformType;

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
pub fn glow_map_core_transform<P, I, V>(core: shader::Core<P, I, V>) -> shader::Core<P, I, V> {
    let fragment = core.fragment.with_out(f_glow_color(), "vec3(f_color)");

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}

/// Shader core for non-glowing objects. This is necessary because otherwise
/// glowing objects will glow through non-glowing objects.
///
/// TODO: Figure out if we can have glow be an uniform instead.
pub fn no_glow_map_core_transform<P, I, V>(core: shader::Core<P, I, V>) -> shader::Core<P, I, V> {
    let fragment = core
        .fragment
        .with_out(f_glow_color(), "vec3(0.0, 0.0, 0.0)");

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}

/// Shader core for blurring the glow texture.
pub fn blur_core() -> shader::Core<(), (), screen_quad::Vertex> {
    let vertex = shader::VertexCore::empty()
        .with_out(shader::defs::v_tex_coord(), "tex_coord")
        .with_out_expr(shader::defs::V_POSITION, "position");

    let defs = "
		float weight[5] = float[] (0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
	";

    let body = "
        // Size of a single texel (0 is the LOD parameter here)
        vec2 texel_size = 1.0 / textureSize(glow_texture, 0);

        // Center fragment contribution
        vec3 blur_result = texture(glow_texture, v_tex_coord).rgb * weight[0];

        // Note that this if is not a problem, since it depends on a uniform only, i.e. it is 
        // constant during the draw call.
        //
        // See also:
        // https://stackoverflow.com/questions/37827216/do-conditional-statements-slow-down-shaders
        if (horizontal) {
            for (int i = 1; i < 5; ++i) {
                blur_result += texture(
                    glow_texture,
                    v_tex_coord + vec2(texel_size.x * i, 0.0)
                ).rgb * weight[i];

                blur_result += texture(
                    glow_texture,
                    v_tex_coord - vec2(texel_size.x * i, 0.0)
                ).rgb * weight[i];
            }
        } else {
            for (int i = 1; i < 5; ++i) {
                blur_result += texture(
                    glow_texture,
                    v_tex_coord + vec2(0.0, texel_size.y * i)
                ).rgb * weight[i];

                blur_result += texture(
                    glow_texture,
                    v_tex_coord - vec2(0.0, texel_size.y * i)
                ).rgb * weight[i];
            }
        }
    ";

    let fragment = shader::FragmentCore::empty()
        .with_extra_uniform("horizontal", UniformType::Bool)
        .with_extra_uniform("glow_texture", UniformType::Sampler2d)
        .with_in_def(shader::defs::v_tex_coord())
        .with_defs(defs)
        .with_body(body)
        .with_out(shader::defs::f_color(), "vec4(blur_result, 1.0)");

    shader::Core { vertex, fragment }
}

/// Shader core for composing the glow texture with the scene texture.
pub fn composition_core_transform(
    core: shader::Core<(), (), screen_quad::Vertex>,
) -> shader::Core<(), (), screen_quad::Vertex> {
    assert!(
        core.fragment.has_in(shader::defs::V_TEX_COORD),
        "FragmentCore needs V_TEX_COORD input for glow composition pass"
    );
    assert!(
        core.fragment.has_out(shader::defs::F_COLOR),
        "FragmentCore needs F_COLOR output for glow composition pass"
    );

    let fragment = core
        .fragment
        .with_extra_uniform("glow_texture", UniformType::Sampler2d)
        .with_out_expr(
            shader::defs::F_COLOR,
            "f_color + vec4(texture(glow_texture, v_tex_coord).rgb, 0.0)",
        );

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}
