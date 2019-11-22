mod shader;

use log::info;

use glium::{uniform, Texture2d, Program, Surface};
use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Sampler, SamplerWrapFunction};

use crate::render::{ScreenQuad, DrawError};

pub use crate::render::CreationError;

#[derive(Debug, Clone)]
pub struct Config {
}

impl Default for Config {
    fn default() -> Self {
        Self {}
    }
}

pub struct FXAA {
    program: Program,
    screen_quad: ScreenQuad,
}

impl FXAA {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
    ) -> Result<Self, CreationError> {
        info!("Creating FXAA program");
        let program = shader::postprocessing_core().build_program(facade)?;

        info!("Creating screen quad");
        let screen_quad = ScreenQuad::create(facade)?;

        Ok(FXAA {
            program,
            screen_quad,
        })
    }

    pub fn run<F: glium::backend::Facade, S: Surface>(
        &self,
        facade: &F,
        texture: &Texture2d,
        target: &mut S,
    ) -> Result<(), DrawError> {
        let texture_map = Sampler::new(texture)
            .magnify_filter(MagnifySamplerFilter::Linear)
            .minify_filter(MinifySamplerFilter::Linear)
            .wrap_function(SamplerWrapFunction::Clamp);

        target.draw(
            &self.screen_quad.vertex_buffer,
            &self.screen_quad.index_buffer,
            &self.program,
            &uniform! {
                texture: texture_map,
            },
            &Default::default(),
        )?;

        Ok(())
    }
}
