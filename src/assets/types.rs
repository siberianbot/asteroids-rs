use glam::{Mat4, Vec2, Vec3};
use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex as VertexTrait};

/// Type of vertex
#[derive(Clone, Default, BufferContents, VertexTrait)]
#[repr(C)]
pub struct Vertex {
    /// Position of vertex
    #[format(R32G32_SFLOAT)]
    #[name("in_position")]
    pub position: Vec2,
}

/// Type of model data
#[derive(Clone, BufferContents)]
#[repr(C)]
pub struct Model {
    pub color: Vec3,
    pub matrix: Mat4,
}
