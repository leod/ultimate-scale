pub mod machine;
pub mod wind;

use rendology::pipeline::CreationError;
use rendology::{
    basic_obj, BasicObj, Instancing, InstancingMode, Light, PlainScenePass, RenderList,
    ShadedScenePass, ShadedScenePassSetup, ShadowPass,
};

#[derive(Default)]
pub struct Stage {
    pub solid: basic_obj::RenderList<basic_obj::Instance>,
    pub solid_glow: basic_obj::RenderList<basic_obj::Instance>,
    pub wind: RenderList<wind::Instance>,

    pub lights: Vec<Light>,

    pub plain: basic_obj::RenderList<basic_obj::Instance>,

    /// Screen-space stuff.
    pub ortho: basic_obj::RenderList<basic_obj::Instance>,
}

#[derive(Clone)]
pub struct Context {
    pub rendology: rendology::Context,
    pub tick_progress: f32,
}

impl Stage {
    pub fn clear(&mut self) {
        self.solid.clear();
        self.solid_glow.clear();
        self.wind.clear();
        self.lights.clear();
        self.plain.clear();
        self.ortho.clear();
    }
}

pub struct Pipeline {
    basic_obj_resources: basic_obj::Resources,

    rendology: rendology::Pipeline,

    solid_shadow_pass: Option<ShadowPass<basic_obj::Core>>,
    solid_scene_pass: ShadedScenePass<basic_obj::Core>,
    solid_glow_scene_pass: ShadedScenePass<basic_obj::Core>,
    wind_scene_pass: ShadedScenePass<wind::Core>,
    plain_scene_pass: PlainScenePass<basic_obj::Core>,

    solid_instancing: basic_obj::Instancing<basic_obj::Instance>,
    solid_glow_instancing: basic_obj::Instancing<basic_obj::Instance>,
    wind_instancing: Instancing<wind::Instance>,
    plain_instancing: basic_obj::Instancing<basic_obj::Instance>,
}

impl Pipeline {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &rendology::Config,
        target_size: (u32, u32),
    ) -> Result<Self, CreationError> {
        let basic_obj_resources = basic_obj::Resources::create(facade)?;

        let rendology = rendology::Pipeline::create(facade, config, target_size)?;

        let solid_shadow_pass =
            rendology.create_shadow_pass(facade, basic_obj::Core, InstancingMode::Vertex)?;
        let solid_scene_pass = rendology.create_shaded_scene_pass(
            facade,
            basic_obj::Core,
            InstancingMode::Vertex,
            ShadedScenePassSetup {
                draw_shadowed: true,
                draw_glowing: false,
            },
        )?;
        let solid_glow_scene_pass = rendology.create_shaded_scene_pass(
            facade,
            basic_obj::Core,
            InstancingMode::Vertex,
            ShadedScenePassSetup {
                draw_shadowed: true,
                draw_glowing: true,
            },
        )?;
        let wind_scene_pass = rendology.create_shaded_scene_pass(
            facade,
            wind::Core,
            InstancingMode::Vertex,
            ShadedScenePassSetup {
                draw_shadowed: true,
                draw_glowing: true,
            },
        )?;
        let plain_scene_pass =
            rendology.create_plain_scene_pass(facade, basic_obj::Core, InstancingMode::Vertex)?;

        let solid_instancing = basic_obj::Instancing::create(facade)?;
        let solid_glow_instancing = basic_obj::Instancing::create(facade)?;
        let wind_instancing = Instancing::create(facade)?;
        let plain_instancing = basic_obj::Instancing::create(facade)?;

        Ok(Self {
            basic_obj_resources,
            rendology,
            solid_shadow_pass,
            solid_scene_pass,
            solid_glow_scene_pass,
            wind_scene_pass,
            plain_scene_pass,
            solid_instancing,
            solid_glow_instancing,
            wind_instancing,
            plain_instancing,
        })
    }

    pub fn draw_frame<F: glium::backend::Facade, S: glium::Surface>(
        &mut self,
        facade: &F,
        context: &Context,
        stage: &Stage,
        target: &mut S,
    ) -> Result<(), rendology::DrawError> {
        self.solid_instancing.update(facade, &stage.solid)?;
        self.solid_glow_instancing
            .update(facade, &stage.solid_glow)?;
        self.wind_instancing
            .update(facade, &stage.wind.as_slice())?;
        self.plain_instancing.update(facade, &stage.plain)?;

        let shaded_draw_params = glium::DrawParameters {
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            ..Default::default()
        };
        let plain_draw_params = glium::DrawParameters {
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            depth: glium::Depth {
                test: glium::DepthTest::IfLessOrEqual,
                write: true,
                ..Default::default()
            },
            line_width: Some(2.0),
            ..Default::default()
        };

        let wind_mesh = self.basic_obj_resources.mesh(BasicObj::TessellatedCylinder);

        self.rendology
            .start_frame(facade, context.rendology.clone(), target)?
            .shadow_pass()
            .draw(
                &self.solid_shadow_pass,
                &self.solid_instancing.as_drawable(&self.basic_obj_resources),
                &(),
            )?
            .draw(
                &self.solid_shadow_pass,
                &self
                    .solid_glow_instancing
                    .as_drawable(&self.basic_obj_resources),
                &(),
            )?
            .shaded_scene_pass()
            .draw(
                &self.solid_scene_pass,
                &self.solid_instancing.as_drawable(&self.basic_obj_resources),
                &(),
                &shaded_draw_params,
            )?
            .draw(
                &self.solid_glow_scene_pass,
                &self
                    .solid_glow_instancing
                    .as_drawable(&self.basic_obj_resources),
                &(),
                &shaded_draw_params,
            )?
            .draw(
                &self.wind_scene_pass,
                &self.wind_instancing.as_drawable(wind_mesh),
                &wind::Params {
                    tick_progress: context.tick_progress,
                },
                &shaded_draw_params,
            )?
            .compose(&stage.lights)?
            .plain_scene_pass()
            .draw(
                &self.plain_scene_pass,
                &self.plain_instancing.as_drawable(&self.basic_obj_resources),
                &(),
                &plain_draw_params,
            )?
            .present()?;

        Ok(())
    }
}
