//! Fast approximate anti-aliasing (FXAA) shader.
//!
//! Heavily inspired by:
//! http://blog.simonrodriguez.fr/articles/30-07-2016_implementing_fxaa.html#ref3
//!
//! See also the following for what seems to be a reference implementation:
//! https://gist.github.com/kosua20/0c506b81b3812ac900048059d2383126
//!
//! I think I understand some of the basic logic of how this shader works, but
//! I'm not going to pretend that I understand *why* some of the choices are
//! made in the implementation.
//!
//! The input of FXAA is a screen-sized texture that is to be anti-aliased.
//! This texture is bound using bilinear interpolation. Then, the fragment
//! shader basically performs a long calculation just to determine a sub-texel
//! offset to the texture coordinates. This offset is orthogonal to the edge
//! along which the pixel lies, or zero if the shader determines that there
//! is no edge. Basically, the magnitude of this offset should depend on how
//! close the pixel is to the end of an edge.
//!
//! Here, the actual smoothing happens due to the bilinear interpolation,
//! outside of the shader, based on the sub-texel offset.

use glium::uniforms::UniformType;

use crate::render::{screen_quad, shader};

pub const EXPLORATION_OFFSETS_LOW: &[f32] = &[1.0, 1.5, 2.0, 4.0, 12.0];
pub const EXPLORATION_OFFSETS_MEDIUM: &[f32] = &[1.0, 1.5, 2.0, 2.0, 2.0, 2.0, 4.0, 8.0];
pub const EXPLORATION_OFFSETS_HIGH: &[f32] =
    &[1.0, 1.0, 1.0, 1.0, 1.0, 1.5, 2.0, 2.0, 2.0, 2.0, 4.0, 8.0];

/// Definitions for the fragment shader.
const DEFS: &str = "
    #define EDGE_THRESHOLD 0.125
    #define EDGE_THRESHOLD_MIN 0.0312
    #define SUBPIXEL_QUALITY 0.75

    float rgb2luma(vec3 rgb){
        return dot(rgb, vec3(0.299, 0.587, 0.114));
    }
";

/// Initialization of the fragment shader body.
const BODY_INIT: &str = "
    vec3 color_center = texture(input_texture, v_tex_coord).rgb;

    // Luma at the current fragment.
    float luma_center = rgb2luma(color_center);

    // Luma at the four direct neighbors of the current fragment.
    float luma_down = rgb2luma(
        textureOffset(input_texture, v_tex_coord, ivec2(0, -1)).rgb
    );
    float luma_up = rgb2luma(
        textureOffset(input_texture, v_tex_coord, ivec2(0, 1)).rgb
    );
    float luma_left = rgb2luma(
        textureOffset(input_texture, v_tex_coord, ivec2(-1, 0)).rgb
    );
    float luma_right = rgb2luma(
        textureOffset(input_texture, v_tex_coord, ivec2(1, 0)).rgb
    );

    // Find the maximum and minimum luma around the current fragment.
    float luma_min = min(
        luma_center,
        min(
            min(luma_down, luma_up),
            min(luma_left, luma_right)
        )
    );
    float luma_max = max(
        luma_center,
        max(
            max(luma_down, luma_up),
            max(luma_left, luma_right)
        )
    );

    // How much variation is there in luma across the samples?
    float luma_range = luma_max - luma_min;

    // There are two situations in which we won't apply AA:
    // 1) The variation in luma is lower than a minimal threshold, or
    // 2) We are in a really dark area.
    if (luma_range < max(EDGE_THRESHOLD_MIN, luma_max * EDGE_THRESHOLD)) {
        f_color = vec4(color_center, 1.0);
        return;
    }

    // Okay, so we will apply AA. Now, we first look at the luma of the four
    // diagonal neighbors.
    float luma_down_left = rgb2luma(
        textureOffset(input_texture, v_tex_coord, ivec2(-1, -1)).rgb
    );
    float luma_up_right = rgb2luma(
        textureOffset(input_texture, v_tex_coord, ivec2(1, 1)).rgb
    );
    float luma_up_left = rgb2luma(
        textureOffset(input_texture, v_tex_coord, ivec2(-1, 1)).rgb
    );
    float luma_down_right = rgb2luma(
        textureOffset(input_texture, v_tex_coord, ivec2(1, -1)).rgb
    );

    // Next, we try to determine if the edge is more horizontal-ish or more
    // vertical-ish. 

    // The following are just some intermediary variables as in the tutorial.
    // I assume GLSL compilers would by now be able to handle such
    // optimizations, but who knows.
    float luma_down_up = luma_down + luma_up;
    float luma_left_right = luma_left + luma_right;
    float luma_left_corners = luma_down_left + luma_up_left;
    float luma_down_corners = luma_down_left + luma_down_right;
    float luma_right_corners = luma_down_right + luma_up_right;
    float luma_up_corners = luma_up_right + luma_up_left;

    // To determine the edge direction, take sums of deltas and then compare.
    //
    // The basic idea, I think, is this. Consider the horizontal: Is there a
    // larger luma difference going up than down, or vice versa? For example, we
    // may have a small difference going down, but a large difference going up.
    // Then we probably have a horizontal edge, and up there is something else.
    //
    // The delta of differences is calculated for the left corners, the center,
    // and the right corners and then summed.
    //
    // Note that this formula gives a higher weight to non-corner deltas.
    // Q: Why? Just because they are closer to the center?
    float edge_horizontal = abs(luma_left_corners - 2.0 * luma_left)
        + 2.0 * abs(luma_down_up - 2.0 * luma_center)
        + abs(luma_right_corners - 2.0 * luma_right);

    // An analogous value is calculated on the vertical.
    float edge_vertical = abs(luma_up_corners - 2.0 * luma_up)
        + 2.0 * abs(luma_left_right - 2.0 * luma_center)
        + abs(luma_down_corners - 2.0 * luma_down);

    bool is_horizontal = edge_horizontal >= edge_vertical;

    // Now we have what is called edge direction in the tutorial, but we still
    // need to find out the edge orientation. For example, given a horizontal
    // edge, is the border up, or down? Again, we just compare differences.
    float luma_neg = is_horizontal ? luma_down : luma_left;
    float luma_pos = is_horizontal ? luma_up : luma_right;

    float delta_neg = abs(luma_neg - luma_center);
    float delta_pos = abs(luma_pos - luma_center);

    bool is_neg_steepest = delta_neg >= delta_pos;

    // For edge exploration later on, this value will serve as a point of
    // comparison for determining if we have reached the end of the edge.
    //
    // Q: Why exactly a factor of 0.25 here? How does this normalize?
    float diff_scaled = 0.25 * max(delta_neg, delta_pos);

    // Move our focus towards the edge border.
    //
    // See also:
    // http://blog.simonrodriguez.fr/articles/30-07-2016_implementing_fxaa/exp3.png)
    vec2 inverse_texture_size = 1.0 / textureSize(input_texture, 0);
    float orientation_offset = is_horizontal ? inverse_texture_size.y : inverse_texture_size.x;
    float luma_local_average = 0.0;

    if (is_neg_steepest) {
        // Will need to apply pixel offsets in inverted direction.
        orientation_offset = -orientation_offset;

        luma_local_average = 0.5 * (luma_neg + luma_center);
    } else {
        luma_local_average = 0.5 * (luma_pos + luma_center);
    }

    vec2 oriented_tex_coord = v_tex_coord;

    if (is_horizontal) {
        oriented_tex_coord.y += orientation_offset * 0.5;
    } else {
        oriented_tex_coord.x += orientation_offset * 0.5;
    }
";

/// First iteration and initialization of edge exploration.
const BODY_INIT_LOOP: &str = "
    // Direction in which will move along the edge in the iterations.
    vec2 exploration_offset = is_horizontal
        ? vec2(inverse_texture_size.x, 0.0)
        : vec2(0.0, inverse_texture_size.y);

    // We explore the edge to the 'left' and to the 'right' until we reach the
    // end each.
    vec2 current_tex_coord_1 = oriented_tex_coord - exploration_offset * FIRST_OFFSET;
    vec2 current_tex_coord_2 = oriented_tex_coord + exploration_offset * FIRST_OFFSET;

    // Read the texture at both positions, compare to the scaled difference at
    // the starting point to heuristically check if we have reached the end.
    float luma_at_1 = rgb2luma(
        texture(input_texture, current_tex_coord_1).rgb
    ) - luma_local_average;
    float luma_at_2 = rgb2luma(
        texture(input_texture, current_tex_coord_2).rgb
    ) - luma_local_average;

    bool reached_1 = abs(luma_at_1) >= diff_scaled;
    bool reached_2 = abs(luma_at_2) >= diff_scaled;
    bool reached_both = reached_1 && reached_2;

    // Advance along the sides according to the offset we are given. Only
    // continue exploring unfinished sides.
    if (!reached_1) {
        current_tex_coord_1 -= exploration_offset * SECOND_OFFSET;
    }
    if (!reached_2) {
        current_tex_coord_2 += exploration_offset * SECOND_OFFSET;
    }
";

/// One iteration of the edge exploration loop.
const BODY_ITERATION: &str = "
    if (!reached_both) {
        if (!reached_1) {
            luma_at_1 = rgb2luma(
                texture(input_texture, current_tex_coord_1).rgb
            ) - luma_local_average;
        }
        if (!reached_2) {
            luma_at_2 = rgb2luma(
                texture(input_texture, current_tex_coord_2).rgb
            ) - luma_local_average;
        }

        // As in `BODY_INIT_LOOP`, check the termination condition.
        reached_1 = abs(luma_at_1) >= diff_scaled;
        reached_2 = abs(luma_at_2) >= diff_scaled;
        reached_both = reached_1 && reached_2;

        // Advance along the sides according to the offset we are given. Only
        // continue exploring unfinished sides.
        if (!reached_1) {
            current_tex_coord_1 -= exploration_offset * OFFSET;
        }
        if (!reached_2) {
            current_tex_coord_2 += exploration_offset * OFFSET;
        }
    }
";

/// Finish the calculation of the texture coordinate to use.
const BODY_FINISH: &str = "
    // We now can see how far we went in both directions of the edge.
    float distance_1 = is_horizontal
        ? (v_tex_coord.x - current_tex_coord_1.x)
        : (v_tex_coord.y - current_tex_coord_1.y);
    float distance_2 = is_horizontal
        ? (current_tex_coord_2.x - v_tex_coord.x)
        : (current_tex_coord_2.y - v_tex_coord.y);
    
    float smaller_distance = min(distance_1, distance_2);
    float edge_length = distance_1 + distance_2;

    // Finally, we get the offset we want to apply to the texture coordinates.
    float pixel_offset = -smaller_distance / edge_length + 0.5;

    // Well, not quite, there is apparently one more check we want to make.
    // During edge exploration, we might have gone off the deep end and e.g.
    // moved into an edge with flipped orientation. In that case, just do not
    // apply smoothing.
    // (Remember that we subtracted luma_local_average from luma_at_{1,2}.)
    bool is_direction_1_closer = distance_1 < distance_2;
    bool is_luma_center_smaller = luma_center < luma_local_average;
    bool is_end_smaller = (is_direction_1_closer ? luma_at_1 : luma_at_2) < 0.0;

    pixel_offset = (is_luma_center_smaller != is_end_smaller) ? pixel_offset : 0.0;

    // Sorry about this, but there is one more thing to check so that we
    // correctly handle subpixel aliasing.
    float luma_average = 1.0 / 12.0 * (2.0 * (luma_down_up + luma_left_right)
        + luma_left_corners
        + luma_right_corners
    );

    float sub_pixel_offset_1 = clamp(abs(luma_average - luma_center) / luma_range, 0.0, 1.0);
    float sub_pixel_offset_2 = (-2.0 * sub_pixel_offset_1 + 3.0)
        * sub_pixel_offset_1
        * sub_pixel_offset_1;
    float sub_pixel_offset = sub_pixel_offset_2 * sub_pixel_offset_2 * SUBPIXEL_QUALITY;

    pixel_offset = max(pixel_offset, sub_pixel_offset);

    // Compute the final tex coords according to the offset.
    vec2 final_tex_coord = v_tex_coord;
    if (is_horizontal) {
        final_tex_coord.y += pixel_offset * orientation_offset;
    } else {
        final_tex_coord.x += pixel_offset * orientation_offset;
    }
";

pub fn postprocessing_core(exploration_offsets: &[f32]) -> shader::Core<(), screen_quad::Vertex> {
    let vertex = shader::VertexCore::empty()
        .with_out(shader::defs::v_tex_coord(), "tex_coord")
        .with_out_expr(shader::defs::V_POSITION, "position");

    if exploration_offsets.len() < 3 {
        panic!("exploration_offsets must contain at least three members");
    }

    let first_offset = exploration_offsets[0];
    let second_offset = exploration_offsets[1];
    let remaining_offsets = &exploration_offsets[2..];

    let mut body = BODY_INIT.to_string()
        + &BODY_INIT_LOOP
            .to_string()
            .replace("FIRST_OFFSET", &first_offset.to_string())
            .replace("SECOND_OFFSET", &second_offset.to_string());

    for offset in remaining_offsets {
        body += &BODY_ITERATION
            .to_string()
            .replace("OFFSET", &offset.to_string());
    }

    body += &BODY_FINISH;

    let fragment = shader::FragmentCore::empty()
        .with_extra_uniform("input_texture", UniformType::Sampler2d)
        .with_defs(DEFS)
        .with_in_def(shader::defs::v_tex_coord())
        .with_body(&body)
        .with_out(
            shader::defs::f_color(),
            "vec4(texture(input_texture, final_tex_coord).rgb, 1.0)",
        );

    shader::Core { vertex, fragment }
}
