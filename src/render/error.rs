#[derive(Debug)]
pub enum CreationError {
    Texture(glium::texture::TextureCreationError),
    Program(glium::program::ProgramCreationError),
    VertexBuffer(glium::vertex::BufferCreationError),
    IndexBuffer(glium::index::BufferCreationError),
    IO(std::io::Error),
}

impl From<glium::texture::TextureCreationError> for CreationError {
    fn from(err: glium::texture::TextureCreationError) -> CreationError {
        CreationError::Texture(err)
    }
}

impl From<glium::program::ProgramCreationError> for CreationError {
    fn from(err: glium::program::ProgramCreationError) -> CreationError {
        CreationError::Program(err)
    }
}

impl From<glium::vertex::BufferCreationError> for CreationError {
    fn from(err: glium::vertex::BufferCreationError) -> CreationError {
        CreationError::VertexBuffer(err)
    }
}

impl From<glium::index::BufferCreationError> for CreationError {
    fn from(err: glium::index::BufferCreationError) -> CreationError {
        CreationError::IndexBuffer(err)
    }
}

impl From<std::io::Error> for CreationError {
    fn from(err: std::io::Error) -> CreationError {
        CreationError::IO(err)
    }
}

#[derive(Debug)]
pub enum DrawError {
    Creation(CreationError),
    Draw(glium::DrawError),
    FramebufferValidation(glium::framebuffer::ValidationError),
    InstancingNotSupported,
}

impl From<CreationError> for DrawError {
    fn from(err: CreationError) -> DrawError {
        DrawError::Creation(err)
    }
}

impl From<glium::DrawError> for DrawError {
    fn from(err: glium::DrawError) -> DrawError {
        DrawError::Draw(err)
    }
}

impl From<glium::framebuffer::ValidationError> for DrawError {
    fn from(err: glium::framebuffer::ValidationError) -> DrawError {
        DrawError::FramebufferValidation(err)
    }
}
