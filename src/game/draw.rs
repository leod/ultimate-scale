use nalgebra as na;

use rendology::Camera;

use crate::config::Config;
use crate::exec::TickTime;
use crate::render;

pub struct Input<'a> {
    pub recreate_pipeline: Option<rendology::Config>,
    pub stage: &'a render::Stage,
    pub context: render::Context,
}

pub type Output = ();

pub struct Draw {
    render_pipeline: render::Pipeline,
}

impl Draw {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
    ) -> Result<Self, CreationError> {
        // TODO: Account for DPI in initialization
        let render_pipeline = render::Pipeline::create(
            facade,
            &config.render_pipeline,
            config.view.window_size.into(),
        )
        .map_err(CreationError::RenderPipeline)?;

        Ok(Draw { render_pipeline })
    }

    pub fn draw<F: glium::backend::Facade, S: glium::Surface>(
        &mut self,
        facade: &F,
        input: &Input,
        target: &mut S,
    ) -> Result<(), rendology::DrawError> {
        self.render_pipeline
            .draw_frame(facade, &input.context, input.stage, target)
    }
}

#[derive(Debug)]
pub enum CreationError {
    RenderPipeline(rendology::pipeline::CreationError),
}
