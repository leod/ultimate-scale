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
