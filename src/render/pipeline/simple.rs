use glium::uniforms::UniformType;

use crate::render::pipeline::{Context, DefaultInstanceParams, InstanceParams};
use crate::render::{object, screen_quad, shader};

pub fn plain_scene_core() -> shader::Core<(Context, DefaultInstanceParams), object::Vertex> {
    shader::Core {
        vertex: shader::VertexCore {
            out_defs: vec![shader::v_world_normal_def(), shader::v_world_pos_def()],
            out_exprs: shader_out_exprs! {
                // TODO: Precompute inverse of mat_model if we ever have lots of vertices
                shader::V_WORLD_NORMAL => "normalize(transpose(inverse(mat3(mat_model))) * normal)",
                shader::V_WORLD_POS => "mat_model * vec4(position, 1.0)",
                shader::V_POSITION => "mat_projection * mat_view * v_world_pos",
            },
            ..Default::default()
        },
        fragment: shader::FragmentCore {
            out_defs: vec![shader::f_color_def()],
            out_exprs: shader_out_exprs! {
                shader::F_COLOR => "color",
            },
            ..Default::default()
        },
    }
}

pub fn diffuse_scene_core_transform<P: InstanceParams, V: glium::vertex::Vertex>(
    core: shader::Core<P, V>,
) -> shader::Core<P, V> {
    let color_expr = if core.fragment.has_out(shader::F_SHADOW) {
        "vec4((0.3 + f_shadow * diffuse) * f_color.rgb, f_color.a)"
    } else {
        "vec4((0.3 + diffuse) * f_color.rgb, f_color.a)"
    };

    shader::Core {
        vertex: core.vertex,
        fragment: core
            .fragment
            .with_in_def(shader::v_world_normal_def())
            .with_in_def(shader::v_world_pos_def())
            .with_body(
                "
                float ambient = 0.3;
                float diffuse = max(
                    dot(
                        normalize(v_world_normal),
                        normalize(light_pos - v_world_pos.xyz)
                    ),
                    0.05
                );
            ",
            )
            .with_out_expr(shader::F_COLOR, color_expr),
    }
}

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
        extra_uniforms: vec![("color_texture".into(), UniformType::Sampler2d)],
        in_defs: vec![shader::v_tex_coord_def()],
        out_defs: vec![shader::f_color_def()],
        out_exprs: shader_out_exprs! {
            shader::F_COLOR => "vec4(texture(color_texture, v_tex_coord).rgb, 1.0)",
        },
        ..Default::default()
    };

    shader::Core { vertex, fragment }
}

pub fn hdr_composition_core_transform(
    core: shader::Core<(), screen_quad::Vertex>,
) -> shader::Core<(), screen_quad::Vertex> {
    assert!(
        core.fragment.has_out(shader::F_COLOR),
        "FragmentCore needs F_COLOR output for HDR composition pass"
    );

    let defs = "

    ";

    let fragment = core
        .fragment
        .with_defs(defs.into())
        .with_out_expr(shader::F_COLOR, "vec4(vec3(1.0) - exp(-f_color.rgb), 1.0)");

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}
