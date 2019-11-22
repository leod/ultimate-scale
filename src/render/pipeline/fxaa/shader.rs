//! Fast approximate anti-aliasing (FXAA) shader.
//!
//! Heavily inspired by:
//! http://blog.simonrodriguez.fr/articles/30-07-2016_implementing_fxaa.html#ref3
//!
//! See also the following for what seems to be a reference implementation:
//! https://gist.github.com/kosua20/0c506b81b3812ac900048059d2383126
//!
//! I think I understand the basic logic of how this shader works, but I'm not
//! going to pretend that I understand all the minutiae, especially *why*
//! things are done in a certain way and not some other way.

use glium::uniforms::UniformType;

use crate::render::{screen_quad, shader};

pub fn postprocessing_core() -> shader::Core<(), screen_quad::Vertex> {
    let vertex = shader::VertexCore {
        out_defs: vec![shader::v_tex_coord_def()],
        out_exprs: shader_out_exprs! {
            shader::V_TEX_COORD => "tex_coord",
            shader::V_POSITION => "position",
        },
        ..Default::default()
    };

    let fragment = shader::FragmentCore {
        extra_uniforms: vec![("input_texture".into(), UniformType::Sampler2d)],
        in_defs: vec![shader::v_tex_coord_def()],
        out_defs: vec![shader::f_color_def()],
        out_exprs: shader_out_exprs! {
            shader::F_COLOR => "vec4(texture(input_texture, v_tex_coord).rgb, 1.0)",
        },
        ..Default::default()
    };

    shader::Core { vertex, fragment }
}
