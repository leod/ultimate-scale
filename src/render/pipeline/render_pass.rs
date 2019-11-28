use crate::render::pipeline::Context;
use crate::render::shader::{self, ToUniforms};
use crate::render::{screen_quad, DrawError};

pub trait RenderPass {
    fn clear_buffers<F: glium::backend::Facade>(&self, facade: &F) -> Result<(), DrawError>;
}

pub trait ScenePassComponent {
    fn core_transform<P: ToUniforms, V: glium::vertex::Vertex>(
        &self,
        core: shader::Core<(Context, P), V>,
    ) -> shader::Core<(Context, P), V>;

    fn output_textures(&self) -> Vec<(&'static str, &glium::texture::Texture2d)> {
        Vec::new()
    }

    // Ideally, we would define a method here which returns uniforms that
    // need to be passed into the transformed shaders for the pass to work.
    // Unfortunately, that is not easily possible, since glium's `uniform!`
    // macro returns a long nested type. We could use "impl trait in trait"
    // again (as in `ToUniforms`), but this is blocked by the fact that,
    // for texture uniforms, the returned type borrows `self`, so it is
    // actually a generic type!
    //
    // Thus, for now, the methods returning uniforms are defined in the
    // individual passes separately.
    //fn uniforms<'a>(&'a self) -> impl Uniforms<'a>;
}

pub trait CompositionPassComponent {
    fn core_transform(
        &self,
        core: shader::Core<(), screen_quad::Vertex>,
    ) -> shader::Core<(), screen_quad::Vertex>;

    // Due to the same reason as described in `ScenePassComponent`, the uniforms
    // are returned in pass-specific methods.
    //fn uniforms<'a>(&'a self) -> impl Uniforms<'a>;
}
