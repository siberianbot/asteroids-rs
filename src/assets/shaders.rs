/// Entity shader
pub mod entity {

    /// Vertex shader
    pub mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: r"
#version 460

layout (location = 0) in vec2 in_position;

layout (binding = 0) uniform Model {
    vec3 color;
    mat4 matrix;
} model;

layout (location = 0) out vec3 out_color;

void main() {
    gl_Position = model.matrix * vec4(in_position, 0.0, 1.0);

    out_color = model.color;
}
        "
        }
    }

    /// Fragment shader
    pub mod fs {
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
}
