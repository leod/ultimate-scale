use nalgebra as na;

use rendology::{basic_obj, shader, Context, SceneCore};

#[derive(Debug, Clone)]
pub struct Params {
    pub tick_progress: f32,
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub transform: na::Matrix4<f32>,
    pub color: na::Vector4<f32>,
    pub stripe_color: na::Vector4<f32>,
    pub phase: f32,
    pub start: f32,
    pub end: f32,
}

rendology::impl_uniform_input!(
    Params,
    self => {
        params_tick_progress: f32 = self.tick_progress,
    },
);

rendology::impl_instance_input!(
    Instance,
    self => {
        instance_transform: [[f32; 4]; 4] = self.transform,
        instance_color: [f32; 4] = self.color,
        instance_stripe_color: [f32; 4] = self.stripe_color,
        instance_phase: f32 = self.phase,
        instance_start: f32 = self.start,
        instance_end: f32 = self.end,
    },
);

const V_DISCARD: &str = "v_discard";

fn v_discard() -> shader::VertexOutDef {
    (
        (V_DISCARD.into(), glium::uniforms::UniformType::Float),
        shader::VertexOutQualifier::Smooth,
    )
}

const V_COLOR: &str = "v_color";

fn v_color() -> shader::VertexOutDef {
    (
        (V_COLOR.into(), glium::uniforms::UniformType::FloatVec4),
        shader::VertexOutQualifier::Smooth,
    )
}

pub struct Core;

impl SceneCore for Core {
    type Params = Params;
    type Instance = Instance;
    type Vertex = basic_obj::Vertex;

    fn scene_core(&self) -> shader::Core<(Context, Params), Instance, basic_obj::Vertex> {
        let vertex = shader::VertexCore::empty()
            .with_defs(
                "
                const float PI = 3.141592;
                const float radius = 0.04;
                const float scale = 0.0105;
            ",
            )
            .with_out_def(v_discard())
            .with_out_def(v_color())
            .with_body(
                "
                float angle = (position.x + 0.5) * PI
                    + params_tick_progress * PI / 2.0
                    + instance_phase;

                float rot_s = sin(angle);
                float rot_c = cos(angle);
                mat2 rot_m = mat2(rot_c, -rot_s, rot_s, rot_c);

                vec3 scaled_pos = position;
                scaled_pos.yz *= scale;
                scaled_pos.z += radius;

                vec3 rot_normal = normal;
                scaled_pos.yz = rot_m * scaled_pos.yz;
                rot_normal.yz = rot_m * rot_normal.yz;

                float x = 0.5 - position.x;

                if (x < instance_start || x > instance_end || instance_start == instance_end)
                    v_discard = 1.0;
                else
                    v_discard = 0.0;

                if (x < params_tick_progress && x > params_tick_progress - 0.3)
                    v_color = instance_stripe_color;
                else if (instance_end == 1.0 && x > 0.7 + params_tick_progress)
                    v_color = instance_stripe_color;
                else
                    v_color = instance_color;
            ",
            )
            .with_out(
                shader::defs::v_world_normal(),
                "normalize(transpose(inverse(mat3(instance_transform))) * rot_normal)",
            )
            .with_out(
                shader::defs::v_world_pos(),
                "instance_transform * vec4(scaled_pos, 1.0)",
            )
            .with_out_expr(
                shader::defs::V_POSITION,
                "context_camera_projection * context_camera_view * v_world_pos",
            );

        let fragment = shader::FragmentCore::empty()
            .with_in_def(v_discard())
            .with_in_def(v_color())
            .with_body(
                "
                if (v_discard >= 0.5)
                    discard;
            ",
            )
            .with_out(shader::defs::f_color(), "v_color");

        shader::Core { vertex, fragment }
    }
}
