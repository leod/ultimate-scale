use nalgebra as na;

use rendology::{shader, Context, CoreInput, CreationError, Mesh, SceneCore};

const SCALE: f32 = 5.0;

#[derive(Clone, Debug)]
pub struct Instance {
    pub size: na::Vector2<f32>,
}

rendology::impl_instance_input!(
    Instance,
    self => {
        instance_size: [f32; 2] = self.size,
    }
);

#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
}

glium::implement_vertex!(Vertex, position);

pub fn create_mesh<F: glium::backend::Facade>(facade: &F) -> Result<Mesh<Vertex>, CreationError> {
    let positions = vec![
        [-1.0, -1.0, 0.0],
        [1.0, -1.0, 0.0],
        [1.0, 1.0, 0.0],
        [-1.0, 1.0, 0.0],
    ];

    let vertices = positions
        .iter()
        .map(|&p| Vertex { position: p })
        .collect::<Vec<_>>();

    let indices = vec![0, 1, 2, 2, 3, 0];

    Mesh::create_with_indices(
        facade,
        glium::index::PrimitiveType::TrianglesList,
        &vertices,
        &indices,
    )
}

pub struct Core;

impl CoreInput for Core {
    type Params = ();
    type Instance = Instance;
    type Vertex = Vertex;
}

pub const V_SIZE: (&str, shader::VertexOutDef) = (
    "v_size",
    shader::VertexOutDef(shader::Type::FloatVec2, shader::VertexOutQualifier::Flat),
);

impl SceneCore for Core {
    fn scene_core(&self) -> shader::Core<(Context, ()), Instance, Vertex> {
        let vertex = shader::VertexCore::empty()
            .with_out(shader::defs::V_WORLD_NORMAL, "vec3(0, 0, 1)")
            .with_out(
                shader::defs::V_WORLD_POS,
                &format!("vec4(vec3(instance_size, 1.0) * position * {}, 1.0)", SCALE),
            )
            .with_out(
                shader::defs::V_POS,
                "context_camera_projection * context_camera_view * v_world_pos",
            )
            .with_out(V_SIZE, "instance_size");

        let defs = "
            vec3 color(vec4 world_pos, vec2 size) {
                if (world_pos.x >= 0.0
                    && world_pos.x <= size.x
                    && world_pos.y >= 0.0
                    && world_pos.y <= size.y) 
                {
                    vec2 pos = floor(world_pos.xy);
                    return mix(
                        vec3(42.9, 60.8, 72.2),
                        vec3(52.9, 80.8, 92.2),
                        mod(pos.x + pos.y, 2.0)
                    ) / 255.0;
                } else if (world_pos.x >= -0.2
                    && world_pos.x <= size.x + 0.2
                    && world_pos.y >= -0.2
                    && world_pos.y <= size.y + 0.2)
                {
                    return vec3(0.2, 0.2, 0.2);
                } else {
                    //return vec3(0.2, 0.2, 0.2);
                    return vec3(0.56, 0.87, 0.98);
                }
            }
        ";

        let fragment = shader::FragmentCore::empty()
            .with_in_def(shader::defs::V_WORLD_POS)
            .with_in_def(V_SIZE)
            .with_defs(defs)
            .with_out(
                shader::defs::F_COLOR,
                "vec4(color(v_world_pos, v_size), 1.0)",
            );

        shader::Core { vertex, fragment }
    }
}
