use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, atomic::Ordering},
};

use glam::Vec2;

use crate::{
    dispatch::{Dispatcher, Event, Sender},
    ecs::ECS,
    entity::{Entity, EntityId},
    game::{Game, State, entities::ASTEROID_SEGMENTS},
    physics,
    worker::Worker,
};

type Collider = [Vec2; 3];

fn translate_rotate(collider: &Collider, position: Vec2, rotation: f32) -> Collider {
    let sin_cos = rotation.sin_cos().into();

    [
        collider[0].rotate(sin_cos) + position,
        collider[1].rotate(sin_cos) + position,
        collider[2].rotate(sin_cos) + position,
    ]
}

fn point_collider_test(point: Vec2, collider: &Collider) -> bool {
    fn bar(p1: Vec2, p2: Vec2, p3: Vec2) -> f32 {
        (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
    }

    let d1 = bar(point, collider[0], collider[1]);
    let d2 = bar(point, collider[1], collider[2]);
    let d3 = bar(point, collider[2], collider[0]);

    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;

    !(has_neg && has_pos)
}

fn collision_test(left: &Collider, right: &Collider) -> bool {
    left.iter()
        .copied()
        .any(|point| point_collider_test(point, right))
        || right
            .iter()
            .copied()
            .any(|point| point_collider_test(point, left))
}

struct ColliderGroup {
    position: Vec2,
    rotation: f32,
    radius: f32,
    colliders: Vec<Collider>,
}

const SPACECRAFT_COLLIDERS: &[Collider] = &[[
    Vec2::new(0.0, 0.5),
    Vec2::new(0.35355339, -0.35355339),
    Vec2::new(-0.35355339, -0.35355339),
]];

const BULLET_COLLIDER_SIZE: f32 = 0.001;
const BULLET_COLLIDERS: &[Collider] = &[
    [
        Vec2::new(BULLET_COLLIDER_SIZE, BULLET_COLLIDER_SIZE),
        Vec2::new(-BULLET_COLLIDER_SIZE, BULLET_COLLIDER_SIZE),
        Vec2::new(-BULLET_COLLIDER_SIZE, -BULLET_COLLIDER_SIZE),
    ],
    [
        Vec2::new(BULLET_COLLIDER_SIZE, BULLET_COLLIDER_SIZE),
        Vec2::new(-BULLET_COLLIDER_SIZE, -BULLET_COLLIDER_SIZE),
        Vec2::new(BULLET_COLLIDER_SIZE, -BULLET_COLLIDER_SIZE),
    ],
];

const DEFAULT_RADIUS: f32 = 1.0;
const ASTEROID_ADDITIONAL_RADIUS: f32 = 2.0;

type Collision = BTreeSet<EntityId>;

pub struct Physics {
    event_sender: Sender<Event>,
    entities: Arc<ECS>,
    colliders: Mutex<BTreeMap<EntityId, ColliderGroup>>,
    collisions: Mutex<BTreeSet<Collision>>,
}

impl Physics {
    pub fn new(event_dispatcher: &Dispatcher<Event>, game: &Game) -> Worker {
        let physics = Physics {
            event_sender: event_dispatcher.create_sender(),
            entities: game.ecs(),
            colliders: Default::default(),
            collisions: Default::default(),
        };

        let physics = Arc::new(physics);

        {
            let physics = physics.clone();

            event_dispatcher.add_handler(move |event| match event {
                Event::EntityCreated(entity_id) => {
                    let group = physics
                        .entities
                        .visit_entity(*entity_id, |entity| match entity {
                            Entity::Spacecraft(spacecraft) => Some(ColliderGroup {
                                position: spacecraft.transform.position,
                                rotation: spacecraft.transform.rotation,
                                radius: DEFAULT_RADIUS,
                                colliders: SPACECRAFT_COLLIDERS.into_iter().copied().collect(),
                            }),

                            Entity::Asteroid(asteroid) => Some(ColliderGroup {
                                position: asteroid.transform.position,
                                rotation: asteroid.transform.rotation,
                                radius: asteroid.asteroid.size + ASTEROID_ADDITIONAL_RADIUS,
                                colliders: (0..ASTEROID_SEGMENTS)
                                    .into_iter()
                                    .map(|index| {
                                        [
                                            Vec2::ZERO,
                                            asteroid.asteroid.body[index],
                                            asteroid.asteroid.body[(index + 1) % ASTEROID_SEGMENTS],
                                        ]
                                    })
                                    .collect(),
                            }),

                            Entity::Bullet(bullet) => Some(ColliderGroup {
                                position: bullet.transform.position,
                                rotation: 0.0,
                                radius: DEFAULT_RADIUS,
                                colliders: BULLET_COLLIDERS.into_iter().copied().collect(),
                            }),

                            _ => None,
                        })
                        .flatten();

                    if let Some(group) = group {
                        physics.colliders.lock().unwrap().insert(*entity_id, group);
                    }
                }

                Event::EntityDestroyed(entity_id) => {
                    physics.colliders.lock().unwrap().remove(entity_id);
                }

                _ => {}
            });
        }

        let worker = {
            let physics = physics.clone();

            Worker::spawn("Physics", move |alive| {
                while alive.load(Ordering::Relaxed) {
                    physics.update();
                    physics.resolve();
                }
            })
        };

        worker
    }

    fn update(&self) {
        let mut groups = self.colliders.lock().unwrap();

        for (entity_id, group) in groups.iter_mut() {
            let position_rotation = self
                .entities
                .visit_entity(*entity_id, |entity| match entity {
                    Entity::Spacecraft(spacecraft) => {
                        (spacecraft.transform.position, spacecraft.transform.rotation)
                    }
                    Entity::Asteroid(asteroid) => {
                        (asteroid.transform.position, asteroid.transform.rotation)
                    }
                    Entity::Bullet(bullet) => (bullet.transform.position, 0.0),

                    _ => unreachable!("entity is not supported for collision detection"),
                });

            if let Some((position, rotation)) = position_rotation {
                group.position = position;
                group.rotation = rotation;
            }
        }
    }

    fn resolve(&self) {
        // TODO: optimize -- too much vars per collider

        struct Item {
            entity_id: EntityId,
            radius: f32,
            position: Vec2,
            collider: Collider,
        }

        let groups = self.colliders.lock().unwrap();

        let mut collisions = self.collisions.lock().unwrap();
        let current_collisions = groups
            .iter()
            .flat_map(|(entity_id, group)| {
                group.colliders.iter().map(|collider| Item {
                    entity_id: *entity_id,
                    radius: group.radius,
                    position: group.position,
                    collider: translate_rotate(collider, group.position, group.rotation),
                })
            })
            .flat_map(|left| {
                groups
                    .iter()
                    .flat_map(|(entity_id, group)| {
                        group.colliders.iter().map(|collider| Item {
                            entity_id: *entity_id,
                            radius: group.radius,
                            position: group.position,
                            collider: translate_rotate(collider, group.position, group.rotation),
                        })
                    })
                    .filter(move |right| left.entity_id < right.entity_id)
                    .filter(move |right| {
                        left.position.distance(right.position) < left.radius + right.radius
                    })
                    .filter(move |right| collision_test(&left.collider, &right.collider))
                    .map(move |right| Collision::from([left.entity_id, right.entity_id]))
            })
            .collect::<BTreeSet<_>>();

        for collision in current_collisions.difference(&collisions).cloned() {
            self.event_sender.send(Event::CollisionStarted(collision));
        }

        for collision in collisions.difference(&current_collisions).cloned() {
            self.event_sender.send(Event::CollisionFinished(collision));
        }

        *collisions = current_collisions;
    }
}
