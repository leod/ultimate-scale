use glium::uniforms::UniformType;

use crate::render::object::Vertex;
use crate::render::pipeline::{Context, DefaultInstanceParams, InstanceParams};
use crate::render::shader;

pub fn plain_core() -> shader::Core<(Context, DefaultInstanceParams), Vertex> {
    shader::Core {
        vertex: shader::VertexCore {
            output_defs: vec![shader::v_world_normal_def(), shader::v_world_pos_def()],
            output_exprs: shader_output_exprs! {
                shader::V_WORLD_NORMAL => "mat3(mat_model) * normal",
                shader::V_WORLD_POS => "mat_model * vec4(position, 1.0)",
                shader::V_POSITION => "mat_projection * mat_view * v_world_pos",
            },
            ..Default::default()
        },
        fragment: shader::FragmentCore {
            outputs: shader_output_exprs! {
                shader::F_COLOR, UniformType::FloatVec4 => "color",
            },
            ..Default::default()
        },
    }
}

pub fn diffuse_core_transform<P: InstanceParams, V: glium::vertex::Vertex>(
    core: shader::Core<P, V>,
) -> shader::Core<P, V> {
    shader::Core {
        vertex: core.vertex,
        fragment: core
            .fragment
            .with_input_def(shader::v_world_normal_def())
            .with_input_def(shader::v_world_pos_def())
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
            .with_updated_output(shader::F_COLOR, |expr| {
                format!("(ambient + diffuse) * ({})", expr)
            }),
    }
}
