use nalgebra as na;

use crate::render::{scene, Camera, RenderList};

#[derive(Debug, Clone)]
pub struct Context {
    pub camera: Camera,
    pub elapsed_time_secs: f32,
    pub tick_progress: f32,
    pub main_light_pos: na::Point3<f32>,
    pub main_light_center: na::Point3<f32>,
}

impl_uniform_input!(
    Context,
    self => {
        viewport: Vec4 => self.camera.viewport.into(),
        mat_projection: Mat4 => self.camera.projection.into(),
        mat_view: Mat4 => self.camera.view.into(),
        main_light_pos: Vec3 => self.main_light_pos.coords.into(),
        elapsed_time_secs: Float => self.elapsed_time_secs,
        tick_progress: Float => self.tick_progress,
    },
);

#[derive(Debug, Clone)]
pub struct Light {
    pub position: na::Point3<f32>,
    pub attenuation: na::Vector3<f32>,
    pub color: na::Vector3<f32>,
    pub is_main: bool,
    pub radius: f32,
}

impl_uniform_input!(
    Light,
    self => {
        light_position: Vec3 => self.position.coords.into(),
        light_attenuation: Vec3 => self.attenuation.into(),
        light_color: Vec3 => self.color.into(),
        light_is_main: Bool => self.is_main,
        light_radius: Float => self.radius,
    },
);

impl Default for Light {
    fn default() -> Self {
        Self {
            position: na::Point3::origin(),
            attenuation: na::Vector3::zeros(),
            color: na::Vector3::zeros(),
            is_main: false,
            radius: 0.0,
        }
    }
}

#[derive(Default, Clone)]
pub struct RenderLists {
    pub solid: RenderList<scene::model::Params>,
    pub wind: RenderList<scene::wind::Params>,
    pub solid_glow: RenderList<scene::model::Params>,

    /// Transparent instances.
    pub transparent: RenderList<scene::model::Params>,

    /// Non-shadowed instances.
    pub plain: RenderList<scene::model::Params>,

    pub lights: Vec<Light>,

    /// Screen-space stuff.
    pub ortho: RenderList<scene::model::Params>,
}

impl RenderLists {
    pub fn clear(&mut self) {
        self.solid.clear();
        self.wind.clear();
        self.solid_glow.clear();
        self.transparent.clear();
        self.plain.clear();
        self.lights.clear();
        self.ortho.clear();
    }

    pub fn append(&mut self, other: &mut Self) {
        self.solid.instances.append(&mut other.solid.instances);
        self.wind.instances.append(&mut other.wind.instances);
        self.solid_glow
            .instances
            .append(&mut other.solid_glow.instances);
        self.transparent
            .instances
            .append(&mut other.transparent.instances);
        self.plain.instances.append(&mut other.plain.instances);
        self.lights.append(&mut other.lights);
        self.ortho.instances.append(&mut other.ortho.instances);
    }
}
