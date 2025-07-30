/// Model of [crate::game::entities::Spacecraft]
pub mod spacecraft {
    use glam::Vec2;

    use crate::assets::types::Vertex;

    /// List of vertices
    pub const VERTICES: [Vertex; 3] = [
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

    /// List of indices
    pub const INDICES: [u32; 3] = [
        0, 1, 2, //
    ];
}

/// Model of [crate::game::entities::Bullet]
pub mod bullet {
    use glam::Vec2;

    use crate::assets::types::Vertex;

    /// List of vertices
    pub const VERTICES: [Vertex; 9] = [
        Vertex {
            position: Vec2::new(0.0, 0.0),
        },
        Vertex {
            position: Vec2::new(0.0, 0.01),
        },
        Vertex {
            position: Vec2::new(0.005, 0.005),
        },
        Vertex {
            position: Vec2::new(0.01, 0.0),
        },
        Vertex {
            position: Vec2::new(0.005, -0.005),
        },
        Vertex {
            position: Vec2::new(0.0, -0.01),
        },
        Vertex {
            position: Vec2::new(-0.005, -0.005),
        },
        Vertex {
            position: Vec2::new(-0.01, 0.0),
        },
        Vertex {
            position: Vec2::new(-0.005, 0.005),
        },
    ];

    /// List of indices
    pub const INDICES: [u32; 24] = [
        0, 1, 2, //
        0, 2, 3, //
        0, 3, 4, //
        0, 4, 5, //
        0, 5, 6, //
        0, 6, 7, //
        0, 7, 8, //
        0, 8, 1, //
    ];
}
