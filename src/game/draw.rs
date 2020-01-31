use crate::config::Config;
use crate::render;

pub struct Input<'a> {
    pub stage: &'a render::Stage,
    pub context: render::Context,
}

pub struct Draw {
    render_pipeline: render::Pipeline,
}

impl Draw {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &Config,
    ) -> Result<Self, rendology::pipeline::CreationError> {
        // TODO: Account for DPI in initialization
        let render_pipeline = render::Pipeline::create(
            facade,
            &config.render_pipeline,
            config.view.window_size.into(),
        )?;

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

    pub fn clean_up_after_exec(&mut self) {
        self.render_pipeline.clear_particles();
    }
}
