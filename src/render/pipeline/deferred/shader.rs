use glium::uniforms::UniformType;

use crate::render::pipeline::{InstanceParams, Light};
use crate::render::{screen_quad, shader};

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

/// Shader core transform for writing position/normal/color into separate
/// buffers, so that they may be combined in a subsequent pass.
pub fn scene_buffers_core_transform<P: InstanceParams, V: glium::vertex::Vertex>(
    core: shader::Core<P, V>,
) -> shader::Core<P, V> {
    assert!(
        core.vertex.has_out(shader::V_WORLD_POS),
        "VertexCore needs V_WORLD_POS output for deferred shading scene pass"
    );
    assert!(
        core.vertex.has_out(shader::V_WORLD_NORMAL),
        "VertexCore needs V_WORLD_NORMAL output for deferred shading scene pass"
    );
    assert!(
        core.fragment.has_out(shader::F_COLOR),
        "FragmentCore needs F_COLOR output for deferred shading scene pass"
    );

    let color_expr = if core.fragment.has_out(shader::F_SHADOW) {
        // TODO: Write shadow value to a separate buffer?
        "f_shadow * f_color"
    } else {
        "f_color"
    };

    let fragment = core
        .fragment
        .with_in_def(shader::v_world_pos_def())
        .with_in_def(shader::v_world_normal_def())
        .with_out_expr(shader::F_COLOR, color_expr)
        .with_out(f_world_pos_def(), "v_world_pos")
        .with_out(f_world_normal_def(), "vec4(v_world_normal, 0.0)");

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}

/// Shader core for rendering a light source, given the position/normal buffers
/// from the scene pass.
pub fn light_core() -> shader::Core<Light, screen_quad::Vertex> {
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
            ("position_texture".into(), UniformType::Sampler2d),
            ("normal_texture".into(), UniformType::Sampler2d),
        ],
        in_defs: vec![shader::v_tex_coord_def()],
        out_defs: vec![shader::f_color_def()],
        body: "
            vec4 position = texture(position_texture, v_tex_coord);
            vec3 normal = normalize(texture(normal_texture, v_tex_coord).xyz);

            vec3 light_vector = light_position - position.xyz;
            float light_distance = length(light_vector);

            float diffuse = max(dot(normal, light_vector / light_distance), 0.0);

            float attenuation = 1.0 / (
                light_attenuation.x +
                light_attenuation.y * light_distance +
                light_attenuation.z * light_distance * light_distance
            );
            //attenuation *= 1.0 - pow(light_distance / light_radius, 2.0);
            attenuation = max(attenuation, 0.0);

            diffuse *= attenuation;

            float radiance = diffuse;
        "
        .into(),
        out_exprs: shader_out_exprs! {
            shader::F_COLOR => "vec4(light_color * radiance, 1.0)",
        },
        ..Default::default()
    };

    shader::Core { vertex, fragment }
}

/// Composition shader core transform for composing our buffers.
pub fn composition_core_transform(
    core: shader::Core<(), screen_quad::Vertex>,
) -> shader::Core<(), screen_quad::Vertex> {
    assert!(
        core.fragment.has_in(shader::V_TEX_COORD),
        "FragmentCore needs V_TEX_COORD input for deferred shading composition pass"
    );
    assert!(
        core.fragment.has_out(shader::F_COLOR),
        "FragmentCore needs F_COLOR output for deferred shading composition pass"
    );

    let fragment = core
        .fragment
        .with_extra_uniform(("light_texture".into(), UniformType::Sampler2d))
        .with_out_expr(
            shader::F_COLOR,
            "f_color * vec4(texture(light_texture, v_tex_coord).rgb, 1.0)",
        );

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}
