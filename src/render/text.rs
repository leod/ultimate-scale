use nalgebra as na;

use std::{io, fs, path};

use glium_text::{TextSystem, FontTexture, TextDisplay};

pub struct Font {
    system: TextSystem,
    texture: FontTexture,
    window_size: glutin::dpi::LogicalSize,
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

        Ok(Font {
            system,
            texture,
            window_size,
        })
    }

    pub fn on_window_resize(&mut self, new_window_size: glutin::dpi::LogicalSize) {
        self.window_size = new_window_size;
    }

    pub fn draw<S: glium::Surface>(
        pos: na::Vector2<f32>,
        color: na::Vector4<f32>,
        text: &str,
        target: &mut S,
    ) {
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

