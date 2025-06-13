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
pub struct Entity {
    color: Vec3,
    matrix: Mat4,
}

pub mod entity_vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
#version 460

layout (location = 0) in vec2 in_position;

layout (binding = 0) uniform Entity {
    vec3 color;
    mat4 matrix;
} entity;

layout (location = 0) out vec3 out_color;

void main() {
    gl_Position = entity.matrix * vec4(in_position, 0.0, 1.0);

    out_color = entity.color;
}
        "
    }
}

pub mod entity_fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
#version 460

layout (location = 0) in vec3 in_color;

layout (location = 0) out vec4 out_color;

void main() {
    out_color = vec4(in_color, 1.0);
}
        "
    }
}
