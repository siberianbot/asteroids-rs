use std::{f32::consts::PI, ops::RangeInclusive};

use glam::{Mat4, Quat, Vec2, Vec3};

use crate::{
    assets::AssetRef,
    game::physics::{Collider, TriangleCollider},
};

/// Identifier of entity
pub type EntityId = usize;

/// Transformation of an entity
#[derive(Default)]
pub struct TransformComponent {
    /// Vector with position
    pub position: Vec2,
    /// Rotation in radians
    pub rotation: f32,
}

impl TransformComponent {
    /// Construct model matrix from [TransformComponent] data
    pub fn to_model_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            Vec3::ONE,
            Quat::from_rotation_z(-self.rotation),
            Vec3::new(self.position.x, self.position.y, 0.0),
        )
    }
}

/// Applicable movement to an entity
#[derive(Default)]
pub struct MovementComponent {
    /// Velocity vector
    pub velocity: Vec2,
    /// Acceleration vector, applicable to velocity vector
    pub acceleration: Vec2,
    /// Determines that velocity is constant
    pub const_velocity: bool,
}

/// Colliders of an entity
#[derive(Default)]
pub struct ColliderComponent {
    /// List of entity colliders
    pub colliders: Vec<Collider>,
}

/// Component with camera data
pub struct CameraComponent {
    /// Distance from camera center to object
    pub distance: f32,
    /// Identifier of target entity
    pub target: Option<EntityId>,
    /// Determines should camera follow target entity
    pub follow: bool,
}

impl Default for CameraComponent {
    fn default() -> Self {
        Self {
            distance: consts::CAMERA_INITIAL_DISTANCE,
            target: Default::default(),
            follow: true,
        }
    }
}

/// Component with spacecraft data
#[derive(Default)]
pub struct SpacecraftComponent {
    /// Reloading cooldown
    pub cooldown: f32,
}

/// Component with asteroid data
pub struct AsteroidComponent {
    /// Rotation velocity
    pub rotation_velocity: f32,
    /// Size of asteroid
    pub size: f32,
    /// Asteroid body
    pub body: [Vec2; consts::ASTEROID_SEGMENTS_COUNT],
}

impl AsteroidComponent {
    /// INTERNAL: generate body of an asteroid
    fn generate_body_(size: f32) -> [Vec2; consts::ASTEROID_SEGMENTS_COUNT] {
        const ANGULAR_STEP: f32 = 2.0 * PI / consts::ASTEROID_SEGMENTS_COUNT as f32;
        const RADIUS_RANGE: RangeInclusive<f32> = 0.5..=1.0;

        let mut body: [Vec2; consts::ASTEROID_SEGMENTS_COUNT] = Default::default();

        for segment_index in 0..consts::ASTEROID_SEGMENTS_COUNT {
            let radius = size + rand::random_range(RADIUS_RANGE);

            let angle = ANGULAR_STEP * segment_index as f32;
            let (sin, cos) = (angle).sin_cos();

            body[segment_index] = Vec2 {
                x: radius * sin,
                y: radius * cos,
            };
        }

        let center: Vec2 = body.iter().sum();

        body.iter_mut().for_each(|segment| *segment -= center);

        body
    }
}

impl Default for AsteroidComponent {
    fn default() -> Self {
        const ROTATION_VELOCITY_MULTIPLIER_RANGE: RangeInclusive<f32> = -2.0..=2.0;
        const SIZE_RANGE: RangeInclusive<u32> = 1..=4;

        let rotation_velocity = rand::random_range(ROTATION_VELOCITY_MULTIPLIER_RANGE);
        let size = rand::random_range(SIZE_RANGE) as f32;
        let body = Self::generate_body_(size);

        Self {
            rotation_velocity,
            size,
            body,
        }
    }
}

/// Component with bullet data
pub struct BulletComponent {
    /// Identifier of entity that spawned bullet
    pub owner: EntityId,
}

/// Component with data for [crate::rendering::renderer::Renderer]
pub struct RenderComponent {
    /// Reference to mesh asset
    pub mesh: AssetRef,
    /// Reference to pipeline asset
    pub pipeline: AssetRef,
}

/// Camera entity
#[derive(Default)]
pub struct Camera {
    /// Transform
    pub transform: TransformComponent,
    /// Camera data
    pub camera: CameraComponent,
}

impl Camera {
    /// Constructs view matrix from [Camera] data
    pub fn to_view_matrix(&self) -> Mat4 {
        Mat4::look_at_lh(
            Vec3::new(
                self.transform.position.x,
                self.transform.position.y,
                self.camera.distance,
            ),
            Vec3::new(self.transform.position.x, self.transform.position.y, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        )
    }
}

/// Spacecraft entity
pub struct Spacecraft {
    /// Transform
    pub transform: TransformComponent,
    /// Movement
    pub movement: MovementComponent,
    /// Collider
    pub collider: ColliderComponent,
    /// Spacecraft
    pub spacecraft: SpacecraftComponent,
    /// Render data
    pub render: RenderComponent,
}

impl Default for Spacecraft {
    fn default() -> Self {
        Self {
            transform: Default::default(),
            movement: Default::default(),
            collider: ColliderComponent {
                colliders: vec![consts::SPACECRAFT_COLLIDER],
            },
            spacecraft: Default::default(),
            render: RenderComponent {
                mesh: consts::SPACECRAFT_MESH_ASSET_REF.into(),
                pipeline: consts::ENTITY_PIPELINE_ASSET_REF.into(),
            },
        }
    }
}

/// Asteroid entity
pub struct Asteroid {
    /// Transform
    pub transform: TransformComponent,
    /// Movement
    pub movement: MovementComponent,
    /// Collider
    pub collider: ColliderComponent,
    /// Asteroid data
    pub asteroid: AsteroidComponent,
    /// Render data
    pub render: RenderComponent,
}

impl Asteroid {
    /// INTERNAL: generate movement component for asteroid
    fn generate_movement_() -> MovementComponent {
        const VELOCITY_RANGE: RangeInclusive<f32> = 0.25..=4.0;
        const VELOCITY_ANGLE_RANGE: RangeInclusive<f32> = 0.0..=2.0 * PI;

        let velocity = rand::random_range(VELOCITY_RANGE);
        let sin_cos = rand::random_range(VELOCITY_ANGLE_RANGE).sin_cos();

        MovementComponent {
            velocity: velocity * Vec2::ONE.rotate(sin_cos.into()),
            acceleration: Default::default(),
            const_velocity: true,
        }
    }

    /// INTERNAL: generate collider component for asteroid
    fn generate_collider_(asteroid: &AsteroidComponent) -> ColliderComponent {
        let radius = asteroid
            .body
            .iter()
            .map(|segment| segment.distance(Vec2::ZERO))
            .max_by(|l, r| l.total_cmp(r))
            .expect("asteroid has no segments");

        ColliderComponent {
            colliders: (0..consts::ASTEROID_SEGMENTS_COUNT)
                .map(|segment_index| {
                    let next_segment_index = (segment_index + 1) % consts::ASTEROID_SEGMENTS_COUNT;

                    TriangleCollider {
                        center: Vec2::ZERO,
                        vertices: [
                            Vec2::ZERO,
                            asteroid.body[segment_index],
                            asteroid.body[next_segment_index],
                        ],
                        radius,
                    }
                    .into()
                })
                .collect(),
        }
    }

    /// INTERNAL: generate render component for asteroid
    fn generate_render_() -> RenderComponent {
        let random = rand::random::<u32>();

        RenderComponent {
            mesh: format!("{}{}", consts::ASTEROID_MESH_ASSET_REF_PREFIX, random).into(),
            pipeline: consts::ENTITY_PIPELINE_ASSET_REF.into(),
        }
    }
}

impl Default for Asteroid {
    fn default() -> Self {
        const ROTATION_RANGE: RangeInclusive<f32> = 0.0..=2.0 * PI;

        let asteroid: AsteroidComponent = Default::default();

        Self {
            transform: TransformComponent {
                rotation: rand::random_range(ROTATION_RANGE),
                ..Default::default()
            },
            movement: Self::generate_movement_(),
            collider: Self::generate_collider_(&asteroid),
            asteroid,
            render: Self::generate_render_(),
        }
    }
}

/// Bullet entity
pub struct Bullet {
    /// Transform
    pub transform: TransformComponent,
    /// Movement
    pub movement: MovementComponent,
    /// Collider
    pub collider: ColliderComponent,
    /// Bullet data
    pub bullet: BulletComponent,
    /// Render data
    pub render: RenderComponent,
}

impl Default for Bullet {
    fn default() -> Self {
        Self {
            transform: Default::default(),
            movement: Default::default(),
            collider: ColliderComponent {
                colliders: vec![consts::BULLET_COLLIDER],
            },
            bullet: BulletComponent {
                owner: EntityId::MAX,
            },
            render: RenderComponent {
                mesh: consts::BULLET_MESH_ASSET_REF.into(),
                pipeline: consts::ENTITY_PIPELINE_ASSET_REF.into(),
            },
        }
    }
}

/// Entity
///
/// See content of next structures for specific details:
/// * [Camera]
/// * [Spacecraft]
/// * [Asteroid]
/// * [Bullet]
#[non_exhaustive]
pub enum Entity {
    /// Variant of entity with [Camera] entity data
    Camera(Camera),
    /// Variant of entity with [Spacecraft] entity data
    Spacecraft(Spacecraft),
    /// Variant of entity with [Asteroid] entity data
    Asteroid(Asteroid),
    /// Variant of entity with [Bullet] entity data
    Bullet(Bullet),
}

impl Entity {
    /// Gets immutable reference to [TransformComponent]
    pub fn transform(&self) -> &TransformComponent {
        match self {
            Entity::Camera(camera) => &camera.transform,
            Entity::Spacecraft(spacecraft) => &spacecraft.transform,
            Entity::Asteroid(asteroid) => &asteroid.transform,
            Entity::Bullet(bullet) => &bullet.transform,
        }
    }

    /// Gets mutable reference to [TransformComponent]
    pub fn transform_mut(&mut self) -> &mut TransformComponent {
        match self {
            Entity::Camera(camera) => &mut camera.transform,
            Entity::Spacecraft(spacecraft) => &mut spacecraft.transform,
            Entity::Asteroid(asteroid) => &mut asteroid.transform,
            Entity::Bullet(bullet) => &mut bullet.transform,
        }
    }

    /// Gets immutable reference to [MovementComponent]
    pub fn movement(&self) -> Option<&MovementComponent> {
        match self {
            Entity::Spacecraft(spacecraft) => Some(&spacecraft.movement),
            Entity::Asteroid(asteroid) => Some(&asteroid.movement),
            Entity::Bullet(bullet) => Some(&bullet.movement),

            _ => None,
        }
    }

    /// Gets mutable reference to [MovementComponent]
    pub fn movement_mut(&mut self) -> Option<&mut MovementComponent> {
        match self {
            Entity::Spacecraft(spacecraft) => Some(&mut spacecraft.movement),
            Entity::Asteroid(asteroid) => Some(&mut asteroid.movement),
            Entity::Bullet(bullet) => Some(&mut bullet.movement),

            _ => None,
        }
    }

    /// Gets immutable reference to [ColliderComponent]
    pub fn collider(&self) -> Option<&ColliderComponent> {
        match self {
            Entity::Spacecraft(spacecraft) => Some(&spacecraft.collider),
            Entity::Asteroid(asteroid) => Some(&asteroid.collider),
            Entity::Bullet(bullet) => Some(&bullet.collider),

            _ => None,
        }
    }

    /// Gets immutable reference to [CameraComponent]
    pub fn camera(&self) -> Option<&CameraComponent> {
        if let Entity::Camera(camera) = self {
            Some(&camera.camera)
        } else {
            None
        }
    }

    /// Gets mutable reference to [CameraComponent]
    pub fn camera_mut(&mut self) -> Option<&mut CameraComponent> {
        if let Entity::Camera(camera) = self {
            Some(&mut camera.camera)
        } else {
            None
        }
    }

    /// Gets immutable reference to [SpacecraftComponent]
    pub fn spacecraft(&self) -> Option<&SpacecraftComponent> {
        if let Entity::Spacecraft(spacecraft) = self {
            Some(&spacecraft.spacecraft)
        } else {
            None
        }
    }

    /// Gets mutable reference to [SpacecraftComponent]
    pub fn spacecraft_mut(&mut self) -> Option<&mut SpacecraftComponent> {
        if let Entity::Spacecraft(spacecraft) = self {
            Some(&mut spacecraft.spacecraft)
        } else {
            None
        }
    }

    /// Gets immutable reference to [AsteroidComponent]
    pub fn asteroid(&self) -> Option<&AsteroidComponent> {
        if let Entity::Asteroid(asteroid) = self {
            Some(&asteroid.asteroid)
        } else {
            None
        }
    }

    /// Gets immutable reference to [BulletComponent]
    pub fn bullet(&self) -> Option<&BulletComponent> {
        if let Entity::Bullet(bullet) = self {
            Some(&bullet.bullet)
        } else {
            None
        }
    }
}

impl From<Camera> for Entity {
    fn from(value: Camera) -> Self {
        Self::Camera(value)
    }
}

impl From<Spacecraft> for Entity {
    fn from(value: Spacecraft) -> Self {
        Self::Spacecraft(value)
    }
}

impl From<Asteroid> for Entity {
    fn from(value: Asteroid) -> Self {
        Self::Asteroid(value)
    }
}

impl From<Bullet> for Entity {
    fn from(value: Bullet) -> Self {
        Self::Bullet(value)
    }
}

/// Constants
pub mod consts {
    use glam::Vec2;

    use crate::game::physics::{Collider, PointCollider, TriangleCollider};

    /// Reference to general entity pipeline asset
    pub const ENTITY_PIPELINE_ASSET_REF: &str = "pipelines/entity";

    /// Initial distance from object to camera center
    pub const CAMERA_INITIAL_DISTANCE: f32 = 4.0;

    /// Default collider of spacecraft
    pub const SPACECRAFT_COLLIDER: Collider = Collider::Triangle(TriangleCollider {
        center: Vec2::ZERO,
        vertices: [
            Vec2::new(0.0, 0.5),
            Vec2::new(0.35355339, -0.35355339),
            Vec2::new(-0.35355339, -0.35355339),
        ],
        radius: 0.5,
    });

    /// Reference to spacecraft mesh asset
    pub const SPACECRAFT_MESH_ASSET_REF: &str = "meshes/spacecraft";

    /// Prefix of reference to asteroid mesh asset
    pub const ASTEROID_MESH_ASSET_REF_PREFIX: &str = "meshes/asteroids/";

    /// Reference to bullet mesh asset
    pub const BULLET_MESH_ASSET_REF: &str = "meshes/bullet";

    /// Count of segments in single asteroid
    pub const ASTEROID_SEGMENTS_COUNT: usize = 8;

    /// Default collider of bullet
    pub const BULLET_COLLIDER: Collider = Collider::Point(PointCollider {
        center: Vec2::ZERO,
        radius: 0.1,
    });
}
