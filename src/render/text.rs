use nalgebra as na;

use std::{fs, io, path};

use glium_text::{FontTexture, TextSystem};

pub struct Font {
    system: TextSystem,
    texture: FontTexture,
    window_size: glutin::dpi::LogicalSize,
    projection: na::Matrix4<f32>,
}

impl Font {
    pub fn load<F: glium::backend::Facade>(
        facade: &F,
        path: &path::Path,
        window_size: glutin::dpi::LogicalSize,
    ) -> Result<Font, CreationError> {
        let system = TextSystem::new(facade);
        let file = fs::File::open(path)?;
        let texture = FontTexture::new(facade, file, 70)?;
        let projection = Self::orthographic_matrix(window_size);

        Ok(Font {
            system,
            texture,
            window_size,
            projection,
        })
    }

    pub fn on_window_resize(&mut self, new_window_size: glutin::dpi::LogicalSize) {
        self.window_size = new_window_size;
        self.projection = Self::orthographic_matrix(new_window_size);
    }

    pub fn draw<S: glium::Surface>(
        &self,
        pos: na::Vector2<f32>,
        size: f32,
        color: na::Vector4<f32>,
        string: &str,
        target: &mut S,
    ) {
        let sub_trans = na::Matrix4::new_translation(&na::Vector3::new(0.0, -1.0, 0.0));
        let scale = na::Matrix4::new_scaling(size);
        let trans = na::Matrix4::new_translation(&na::Vector3::new(pos.x, self.window_size.height as f32 - pos.y, 0.0));
        let matrix: [[f32; 4]; 4] = (self.projection * trans * scale * sub_trans).into();

        let text = glium_text::TextDisplay::new(&self.system, &self.texture, string);
        glium_text::draw(
            &text,
            &self.system,
            target,
            matrix,
            (color.x, color.y, color.z, color.w),
        );
    }

    fn orthographic_matrix(
        window_size: glutin::dpi::LogicalSize
    ) -> na::Matrix4<f32> {
        na::Matrix4::new_orthographic(
            0.0,
            window_size.width as f32,
            0.0,
            window_size.height as f32,
            -1.0,
            0.0
        )
    }
}

#[derive(Debug)]
pub enum CreationError {
    IOError(io::Error),
    FontTextureError(()),
}

impl From<io::Error> for CreationError {
    fn from(err: io::Error) -> CreationError {
        CreationError::IOError(err)
    }
}

impl From<()> for CreationError {
    fn from(err: ()) -> CreationError {
        CreationError::FontTextureError(err)
    }
}
