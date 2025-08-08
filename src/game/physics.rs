use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use glam::Vec2;

use crate::{
    game::{ecs::ECS, entities::EntityId},
    handle, workers,
};

/// Point collider data
pub struct PointCollider {
    /// Center of the point collider
    pub center: Vec2,
    /// Radius of the collider (also used as activation radius)
    pub radius: f32,
}

impl PointCollider {
    /// INTERNAL: applies transformation to [PointCollider]
    fn transform(&self, position: Vec2) -> PointCollider {
        PointCollider {
            center: position + self.center,
            radius: self.radius,
        }
    }
}

/// Triangle collider data
pub struct TriangleCollider {
    /// Origin of collider
    pub center: Vec2,
    /// Vertices of triangle
    pub vertices: [Vec2; 3],
    /// Activation radius
    pub radius: f32,
}

impl TriangleCollider {
    /// INTERNAL: applies transformation to [TriangleCollider]
    fn transform(&self, position: Vec2, rotation: f32) -> TriangleCollider {
        TriangleCollider {
            center: position + self.center,
            vertices: [
                position + self.vertices[0].rotate(rotation.sin_cos().into()),
                position + self.vertices[1].rotate(rotation.sin_cos().into()),
                position + self.vertices[2].rotate(rotation.sin_cos().into()),
            ],
            radius: self.radius,
        }
    }
}

/// INTERNAL: checks that point lies inside triangle using barycentric coordinates
fn barycentric_triangle_point_test(point: Vec2, triangle: &[Vec2; 3]) -> bool {
    fn determinant(point: Vec2, v1: Vec2, v2: Vec2) -> f32 {
        (point.x - v2.x) * (v1.y - v2.y) - (point.y - v2.y) * (v1.x - v2.x)
    }

    let determinants = [
        determinant(point, triangle[0], triangle[1]),
        determinant(point, triangle[1], triangle[2]),
        determinant(point, triangle[2], triangle[0]),
    ];

    let negative = determinants.iter().any(|d| *d < 0.0);
    let positive = determinants.iter().any(|d| *d > 0.0);

    !(negative && positive)
}

/// INTERNAL: tests collision of two points
fn point_point_collision_test(left: &PointCollider, right: &PointCollider) -> bool {
    let distance = left.center.distance(right.center);

    distance <= left.radius + right.radius
}

/// INTERNAL: tests collision of triangle and point
fn triangle_point_collision_test(left: &TriangleCollider, right: &PointCollider) -> bool {
    let distance = left.center.distance(right.center);

    if distance > left.radius + right.radius {
        return false;
    }

    barycentric_triangle_point_test(right.center, &left.vertices)
}

/// INTERNAL: tests collision of two triangle
fn triangle_triangle_collision_test(left: &TriangleCollider, right: &TriangleCollider) -> bool {
    fn test(left: &TriangleCollider, right: &TriangleCollider) -> bool {
        left.vertices
            .iter()
            .copied()
            .any(|point| barycentric_triangle_point_test(point, &right.vertices))
    }

    let distance = left.center.distance(right.center);

    if distance > left.radius + right.radius {
        return false;
    }

    test(left, right) || test(right, left)
}

/// Collider
///
/// See next structures for specific details:
/// * [PointCollider]
/// * [TriangleCollider]
pub enum Collider {
    /// Variant with [PointCollider] data
    Point(PointCollider),
    /// Variant with [TriangleCollider] data
    Triangle(TriangleCollider),
}

impl Collider {
    /// INTERNAL: applies transformation to [Collider]
    fn transform(&self, position: Vec2, rotation: f32) -> Collider {
        match self {
            Collider::Point(collider) => collider.transform(position).into(),
            Collider::Triangle(collider) => collider.transform(position, rotation).into(),
        }
    }

    /// INTERNAL: performs collision test of two colliders
    fn collision_test(&self, collider: &Collider) -> bool {
        match self {
            Collider::Point(left) => match collider {
                Collider::Point(right) => point_point_collision_test(left, right),
                Collider::Triangle(right) => triangle_point_collision_test(right, left),
            },

            Collider::Triangle(left) => match collider {
                Collider::Point(right) => triangle_point_collision_test(left, right),
                Collider::Triangle(right) => triangle_triangle_collision_test(left, right),
            },
        }
    }
}

impl From<PointCollider> for Collider {
    fn from(value: PointCollider) -> Self {
        Collider::Point(value)
    }
}

impl From<TriangleCollider> for Collider {
    fn from(value: TriangleCollider) -> Self {
        Collider::Triangle(value)
    }
}

/// Detected collision data with involved [EntityId]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Collision(pub EntityId);

/// Physics infrastructure
pub struct Physics {
    ecs: Arc<ECS>,
}

impl Physics {
    /// Creates new instance of [Physics]
    pub fn new(ecs: Arc<ECS>) -> Physics {
        Physics { ecs }
    }

    /// INTERNAL: collects all occurred collisions
    fn collect_collisions(&self) -> BTreeMap<EntityId, BTreeSet<Collision>> {
        let entities = self.ecs.read();

        let iter_entities = || {
            entities
                .iter()
                .filter_map(|(entity_id, entity)| {
                    entity
                        .collider()
                        .map(|collider| (entity_id, entity.transform(), collider))
                })
                .flat_map(|(entity_id, transform, collider)| {
                    collider.colliders.iter().map(move |collider| {
                        (
                            entity_id,
                            collider.transform(transform.position, transform.rotation),
                        )
                    })
                })
        };

        let collisions = iter_entities()
            .flat_map(|(left_entity_id, left_collider)| {
                iter_entities()
                    .filter(move |(right_entity_id, _)| left_entity_id > *right_entity_id)
                    .filter(move |(_, right_collider)| left_collider.collision_test(right_collider))
                    .flat_map(move |(right_entity_id, _)| {
                        [
                            (left_entity_id, Collision(right_entity_id)),
                            (right_entity_id, Collision(left_entity_id)),
                        ]
                    })
            })
            .fold(
                BTreeMap::<_, BTreeSet<_>>::new(),
                |mut map, (entity_id, collision)| {
                    map.entry(entity_id).or_default().insert(collision);

                    map
                },
            );

        collisions
    }

    /// INTERNAL: stores all collisions in [crate::game::entities::ColliderComponent]
    fn store_collisions(&self, collisions: BTreeMap<EntityId, BTreeSet<Collision>>) {
        let mut entities = self.ecs.write();

        for (entity_id, collisions) in collisions {
            entities.modify(entity_id, |entity| {
                entity
                    .collider_mut()
                    .map(|collider| collider.collisions.extend(collisions))
            });
        }
    }
}

/// INTERNAL: Physics worker thread function
fn worker_func(physics: &Physics) {
    let collisions = physics.collect_collisions();

    physics.store_collisions(collisions);
}

/// Spawns physics worker thread
pub fn spawn_worker(workers: &workers::Workers, physics: Physics) -> handle::Handle {
    workers.spawn("Physics", move |token| {
        const UPDATE_RATE: f32 = 1.0 / 120.0;

        let mut last_update = Instant::now();

        while !token.is_cancelled() {
            let elapsed = Instant::now().duration_since(last_update).as_secs_f32();

            worker_func(&physics);

            last_update = Instant::now();

            if elapsed < UPDATE_RATE {
                let duration = Duration::from_secs_f32(UPDATE_RATE - elapsed);

                thread::sleep(duration);
            }
        }
    })
}
