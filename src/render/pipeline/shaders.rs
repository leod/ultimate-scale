use glium::uniforms::UniformType;

use crate::render::screen_quad;
use crate::render::shader::{self, ToUniforms};

pub fn diffuse_scene_core_transform<P: ToUniforms, V: glium::vertex::Vertex>(
    core: shader::Core<P, V>,
) -> shader::Core<P, V> {
    let color_expr = if core.fragment.has_out(shader::defs::F_SHADOW) {
        "vec4((0.3 + f_shadow * diffuse) * f_color.rgb, f_color.a)"
    } else {
        "vec4((0.3 + diffuse) * f_color.rgb, f_color.a)"
    };

    let fragment = core
        .fragment
        .with_in_def(shader::defs::v_world_normal())
        .with_in_def(shader::defs::v_world_pos())
        .with_body(
            "
            float ambient = 0.3;
            float diffuse = max(
                dot(
                    normalize(v_world_normal),
                    normalize(main_light_pos - v_world_pos.xyz)
                ),
                0.05
            );
        ",
        )
        .with_out_expr(shader::defs::F_COLOR, color_expr);

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}

pub fn composition_core() -> shader::Core<(), screen_quad::Vertex> {
    let vertex = shader::VertexCore::empty()
        .with_out(shader::defs::v_tex_coord(), "tex_coord")
        .with_out_expr(shader::defs::V_POSITION, "position");

    let fragment = shader::FragmentCore::empty()
        .with_extra_uniform("color_texture", UniformType::Sampler2d)
        .with_in_def(shader::defs::v_tex_coord())
        .with_out(
            shader::defs::f_color(),
            "vec4(texture(color_texture, v_tex_coord).rgb, 1.0)",
        );

    shader::Core { vertex, fragment }
}

pub fn hdr_composition_core_transform(
    core: shader::Core<(), screen_quad::Vertex>,
) -> shader::Core<(), screen_quad::Vertex> {
    assert!(
        core.fragment.has_out(shader::defs::F_COLOR),
        "FragmentCore needs F_COLOR output for HDR composition pass"
    );

    let fragment = core
        .fragment
        //.with_out_expr(shader::F_COLOR, "vec4(vec3(1.0) - exp(-f_color.rgb), 1.0)");
        .with_out_expr(
            shader::defs::F_COLOR,
            "vec4(vec3(f_color) / (vec3(f_color) + 1.0), 1.0)",
        );

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}

pub fn gamma_correction_composition_core_transform(
    core: shader::Core<(), screen_quad::Vertex>,
    gamma: f32,
) -> shader::Core<(), screen_quad::Vertex> {
    assert!(
        core.fragment.has_out(shader::defs::F_COLOR),
        "FragmentCore needs F_COLOR output for gamma correction composition pass"
    );

    let fragment = core.fragment.with_out_expr(
        shader::defs::F_COLOR,
        &format!("vec4(pow(vec3(f_color), vec3(1.0 / {})), 1.0)", gamma),
    );

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}
