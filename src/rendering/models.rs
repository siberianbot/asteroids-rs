use glam::Vec2;

use crate::rendering::shaders::Vertex;

pub const SPACECRAFT_VERTICES: [Vertex; 3] = [
    Vertex {
        position: Vec2::new(0.0, 0.5),
    },
    Vertex {
        position: Vec2::new(0.35355339, -0.35355339),
    },
    Vertex {
        position: Vec2::new(-0.35355339, -0.35355339),
    },
];

pub const SPACECRAFT_INDICES: [u32; 3] = [0, 1, 2];

pub const ASTEROID_INDICES: [u32; 24] = [
    // TODO: try to enumerate in compile-time by using ASTEROID_SEGMENTS value
    0, 1, 2, //
    0, 2, 3, //
    0, 3, 4, //
    0, 4, 5, //
    0, 5, 6, //
    0, 6, 7, //
    0, 7, 8, //
    0, 8, 1, //
];

pub const BULLET_VERTICES: [Vertex; 1] = [Vertex {
    position: Vec2::new(0.0, 0.0),
}];

pub const BULLET_INDICES: [u32; 1] = [0];
