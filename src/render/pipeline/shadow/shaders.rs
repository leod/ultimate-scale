use glium::uniforms::UniformType;

use crate::render::pipeline::Context;
use crate::render::shader;

/// Shader core for rendering the depth map from the light source's perspective.
pub fn depth_map_core_transform<P, I, V>(core: shader::Core<P, I, V>) -> shader::Core<P, I, V> {
    // Only write depth into the output, discard color output of original core
    let fragment = shader::FragmentCore::default()
        .with_out(shader::defs::f_fragment_depth(), "gl_FragCoord.z");

    shader::Core {
        vertex: core.vertex,
        fragment,
    }
}

/// Shader core for rendering the shadowed scene.
pub fn render_shadowed_core_transform<I, V>(
    core: shader::Core<Context, I, V>,
) -> shader::Core<Context, I, V> {
    assert!(
        core.vertex.has_out(shader::defs::V_WORLD_POS),
        "VertexCore needs V_WORLD_POS output for shadow mapping"
    );
    assert!(
        core.vertex.has_out(shader::defs::V_WORLD_NORMAL),
        "VertexCore needs V_WORLD_NORMAL output for shadow mapping"
    );

    // Position of current vertex in light space
    let v_light_space_pos = (
        ("v_light_space_pos".into(), UniformType::FloatVec4),
        shader::VertexOutQualifier::Smooth,
    );

    let vertex = core
        .vertex
        .with_extra_uniform("mat_light_view_projection", UniformType::FloatMat4)
        .with_out(
            v_light_space_pos.clone(),
            // Bias shadow coord a bit in the direction of the normal --
            // this is a simple fix for a lot of self-shadowing artifacts
            "mat_light_view_projection * (v_world_pos + 0.02 * vec4(v_world_normal, 0.0))",
        );

    let fragment = core
        .fragment
        .with_extra_uniform("shadow_map", UniformType::Sampler2d)
        .with_in_def(shader::defs::v_world_pos())
        .with_in_def(shader::defs::v_world_normal())
        .with_in_def(v_light_space_pos)
        .with_defs(
            "
            float shadow_calculation(vec4 light_space_pos) {
                // main_light_pos uniform is provided by Context.
                vec3 light_dir = normalize(vec3(main_light_pos - v_world_pos.xyz));

                vec3 proj_coords = light_space_pos.xyz / light_space_pos.w;
                proj_coords = proj_coords * 0.5 + 0.5;

                if (dot(light_dir, v_world_normal) < 0.0)
                    return 0.5;

                // TODO: Is there a way to do this on texture-level?
                if (proj_coords.z > 1.0
                    || proj_coords.x < 0.0
                    || proj_coords.x > 1.0
                    || proj_coords.y < 0.0
                    || proj_coords.y > 1.0) {
                    return 1.0;
                }

                float closest_depth = texture(shadow_map, proj_coords.xy).r;
                float current_depth = proj_coords.z;

                return current_depth > closest_depth ? 0.5 : 1.0;
            }
            ",
        )
        .with_out(
            shader::defs::f_shadow(),
            "shadow_calculation(v_light_space_pos)",
        );

    shader::Core { vertex, fragment }
}
