use glam::Vec2;
use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex as VertexTrait};

/// Type of vertex
#[derive(Default, BufferContents, VertexTrait)]
#[repr(C)]
pub struct Vertex {
    /// Position of vertex
    #[format(R32G32_SFLOAT)]
    #[name("in_position")]
    pub position: Vec2,
}
