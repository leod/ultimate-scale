pub mod machine;
pub mod wind;

use nalgebra as na;

use coarse_prof::profile;

use rendology::pipeline::CreationError;
use rendology::{
    basic_obj, line, BasicObj, Camera, Drawable, Instancing, InstancingMode, Light, Mesh,
    PlainScenePass, RenderList, SceneCore, ShadedScenePass, ShadedScenePassSetup, ShadowPass,
};

#[derive(Default)]
pub struct Stage {
    pub solid: basic_obj::RenderList<basic_obj::Instance>,
    pub solid_glow: basic_obj::RenderList<basic_obj::Instance>,
    pub wind: RenderList<wind::Instance>,

    pub lights: Vec<Light>,

    pub plain: basic_obj::RenderList<basic_obj::Instance>,
    pub lines: RenderList<line::Instance>,

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
        self.lines.clear();
        self.ortho.clear();
    }
}

pub struct Pipeline {
    basic_obj_resources: basic_obj::Resources,
    line_mesh: Mesh<line::Point>,
    plain_program: glium::Program,

    rendology: rendology::Pipeline,

    solid_shadow_pass: Option<ShadowPass<basic_obj::Core>>,
    wind_shadow_pass: Option<ShadowPass<wind::Core>>,

    solid_scene_pass: ShadedScenePass<basic_obj::Core>,
    solid_glow_scene_pass: ShadedScenePass<basic_obj::Core>,
    wind_scene_pass: ShadedScenePass<wind::Core>,

    plain_scene_pass: PlainScenePass<basic_obj::Core>,
    line_scene_pass: PlainScenePass<line::Core>,

    solid_instancing: basic_obj::Instancing<basic_obj::Instance>,
    solid_glow_instancing: basic_obj::Instancing<basic_obj::Instance>,
    wind_instancing: Instancing<wind::Instance>,
    plain_instancing: basic_obj::Instancing<basic_obj::Instance>,
    line_instancing: Instancing<line::Instance>,
}

impl Pipeline {
    pub fn create<F: glium::backend::Facade>(
        facade: &F,
        config: &rendology::Config,
        target_size: (u32, u32),
    ) -> Result<Self, CreationError> {
        let basic_obj_resources = basic_obj::Resources::create(facade)?;
        let line_mesh = line::create_mesh(facade)?;
        let plain_program = basic_obj::Core
            .scene_core()
            .build_program(facade, InstancingMode::Uniforms)
            .map_err(|e| CreationError::CreationError(rendology::CreationError::ShaderBuild(e)))?;

        let rendology = rendology::Pipeline::create(facade, config, target_size)?;

        let solid_shadow_pass =
            rendology.create_shadow_pass(facade, basic_obj::Core, InstancingMode::Vertex)?;
        let wind_shadow_pass =
            rendology.create_shadow_pass(facade, wind::Core, InstancingMode::Vertex)?;

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
        let line_scene_pass =
            rendology.create_plain_scene_pass(facade, line::Core, InstancingMode::Vertex)?;

        let solid_instancing = basic_obj::Instancing::create(facade)?;
        let solid_glow_instancing = basic_obj::Instancing::create(facade)?;
        let wind_instancing = Instancing::create(facade)?;
        let plain_instancing = basic_obj::Instancing::create(facade)?;
        let line_instancing = Instancing::create(facade)?;

        Ok(Self {
            basic_obj_resources,
            line_mesh,
            plain_program,
            rendology,
            solid_shadow_pass,
            wind_shadow_pass,
            solid_scene_pass,
            solid_glow_scene_pass,
            wind_scene_pass,
            plain_scene_pass,
            line_scene_pass,
            solid_instancing,
            solid_glow_instancing,
            wind_instancing,
            plain_instancing,
            line_instancing,
        })
    }

    pub fn draw_frame<F: glium::backend::Facade, S: glium::Surface>(
        &mut self,
        facade: &F,
        context: &Context,
        stage: &Stage,
        target: &mut S,
    ) -> Result<(), rendology::DrawError> {
        {
            profile!("update_instances");

            self.solid_instancing.update(facade, &stage.solid)?;
            self.solid_glow_instancing
                .update(facade, &stage.solid_glow)?;
            self.wind_instancing
                .update(facade, &stage.wind.as_slice())?;
            self.plain_instancing.update(facade, &stage.plain)?;
            self.line_instancing
                .update(facade, stage.lines.as_slice())?;
        }

        let scene_offset = Some(glium::draw_parameters::PolygonOffset {
            factor: 1.0,
            units: 1.0,
        });
        let shaded_draw_params = glium::DrawParameters {
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            polygon_offset: scene_offset,
            ..Default::default()
        };
        let plain_draw_params = glium::DrawParameters {
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            depth: glium::Depth {
                test: glium::DepthTest::IfLessOrEqual,
                write: true,
                ..Default::default()
            },
            polygon_offset: scene_offset,
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };
        let line_draw_params = glium::DrawParameters {
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullClockwise,
            depth: glium::Depth {
                test: glium::DepthTest::IfLessOrEqual,
                write: false,
                ..Default::default()
            },
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };

        let wind_color = machine::wind_source_color();
        let wind_stripe_color = machine::wind_stripe_color();
        let wind_params = wind::Params {
            tick_progress: context.tick_progress,
            color: na::Vector4::new(wind_color.x, wind_color.y, wind_color.z, 1.0),
            stripe_color: na::Vector4::new(
                wind_stripe_color.x,
                wind_stripe_color.y,
                wind_stripe_color.z,
                1.0,
            ),
        };
        let wind_mesh = self.basic_obj_resources.mesh(BasicObj::TessellatedCylinder);

        self.rendology
            .start_frame(facade, (0.0, 0.0, 0.0), context.rendology.clone(), target)?
            .shadow_pass()
            .draw(
                &self.solid_shadow_pass,
                &self.solid_instancing.as_drawable(&self.basic_obj_resources),
                &(),
                &shaded_draw_params,
            )?
            .draw(
                &self.solid_shadow_pass,
                &self
                    .solid_glow_instancing
                    .as_drawable(&self.basic_obj_resources),
                &(),
                &shaded_draw_params,
            )?
            .draw(
                &self.wind_shadow_pass,
                &self.wind_instancing.as_drawable(wind_mesh),
                &wind_params,
                &shaded_draw_params,
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
                &wind_params,
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
            .postprocess()?
            .plain_scene_pass()
            .draw(
                &self.line_scene_pass,
                &self.line_instancing.as_drawable(&self.line_mesh),
                &line::Params { feather: 1.0 },
                &line_draw_params,
            )?
            .present()?;

        // Render screen-space stuff on top
        profile!("ortho");

        let ortho_projection = na::Matrix4::new_orthographic(
            0.0,
            context.rendology.camera.viewport_size.x,
            context.rendology.camera.viewport_size.y,
            0.0,
            -10.0,
            10.0,
        );
        let ortho_camera = Camera {
            projection: ortho_projection,
            view: na::Matrix4::identity(),
            ..context.rendology.camera.clone()
        };
        let ortho_render_context = rendology::Context {
            camera: ortho_camera,
            ..context.rendology.clone()
        };
        let ortho_parameters = glium::DrawParameters {
            blend: glium::draw_parameters::Blend::alpha_blending(),
            ..Default::default()
        };
        stage.ortho.as_drawable(&self.basic_obj_resources).draw(
            &self.plain_program,
            &ortho_render_context,
            &ortho_parameters,
            target,
        )?;

        Ok(())
    }
}
