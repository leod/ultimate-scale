use crate::render::object::Vertex;
use crate::render::pipeline::{Context, DefaultInstanceParams, InstanceParams};
use crate::render::shader;

pub fn plain_core() -> shader::Core<(Context, DefaultInstanceParams), Vertex> {
    shader::Core {
        vertex: shader::VertexCore {
            out_defs: vec![shader::v_world_normal_def(), shader::v_world_pos_def()],
            out_exprs: shader_output_exprs! {
                // TODO: Precompute inverse of mat_model if we ever have lots of vertices
                shader::V_WORLD_NORMAL => "normalize(transpose(inverse(mat3(mat_model))) * normal)",
                shader::V_WORLD_POS => "mat_model * vec4(position, 1.0)",
                shader::V_POSITION => "mat_projection * mat_view * v_world_pos",
            },
            ..Default::default()
        },
        fragment: shader::FragmentCore {
            out_defs: vec![shader::f_color_def()],
            out_exprs: shader_output_exprs! {
                shader::F_COLOR => "color",
            },
            ..Default::default()
        },
    }
}

pub fn diffuse_core_transform<P: InstanceParams, V: glium::vertex::Vertex>(
    core: shader::Core<P, V>,
) -> shader::Core<P, V> {
    let color_expr = if core.fragment.has_out(shader::F_SHADOW) {
        "(0.3 + f_shadow * diffuse) * f_color"
    } else {
        "(0.3 + diffuse) * f_color"
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
