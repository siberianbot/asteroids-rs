use glam::{Mat4, Vec2, Vec3};
use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex as VertexTrait};

#[derive(BufferContents, VertexTrait)]
#[repr(C)]
pub struct Vertex {
    #[format(R32G32_SFLOAT)]
    #[name("in_position")]
    pub position: Vec2,
}

#[derive(BufferContents)]
#[repr(C)]
pub struct Model {
    pub color: Vec3,
    pub matrix: Mat4,
}
