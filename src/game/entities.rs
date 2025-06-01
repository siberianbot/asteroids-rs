use std::{
    f32::consts::PI,
    ops::RangeInclusive,
    sync::{Arc, Mutex},
};

use glam::{Vec2, vec2};

use crate::dispatch::{Dispatcher, Event, Sender};

pub type EntityId = usize;

pub const ASTEROID_SEGMENTS: usize = 8;
pub const ASTEROID_SEGMENT_RANGE: RangeInclusive<f32> = 0.75..=1.0;

pub struct Camera {
    pub position: Vec2,
    pub distance: f32,
    pub target: EntityId,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Default::default(),
            distance: 1.0,
            target: Default::default(),
        }
    }
}

pub struct Spacecraft {
    pub position: Vec2,
    pub rotation: f32,
}

impl Default for Spacecraft {
    fn default() -> Self {
        Self {
            position: Default::default(),
            rotation: Default::default(),
        }
    }
}

pub struct Asteroid {
    pub position: Vec2,
    pub rotation: f32,
    pub body: [Vec2; ASTEROID_SEGMENTS],
}

impl Default for Asteroid {
    fn default() -> Self {
        let mut body: [Vec2; ASTEROID_SEGMENTS] = Default::default();

        let mut min = Vec2::ZERO;
        let mut max = Vec2::ZERO;
        let angle_step = 2.0 * PI / ASTEROID_SEGMENTS as f32;

        for segment in 0..ASTEROID_SEGMENTS {
            let radius = rand::random_range(ASTEROID_SEGMENT_RANGE);
            let angle = angle_step * segment as f32;

            let x = radius * angle.sin();
            let y = radius * angle.cos();

            body[segment] = vec2(x, y);

            match x {
                x if x < min.x => min.x = x,
                x if x > max.x => max.x = x,
                _ => {}
            }

            match y {
                y if y < min.y => min.y = y,
                y if y > max.y => max.y = y,
                _ => {}
            }
        }

        let center = (max - min) / 2.0;
        body.iter_mut().for_each(|segment| *segment -= center);

        Self {
            position: Default::default(),
            rotation: Default::default(),
            body,
        }
    }
}

pub enum Entity {
    Camera(Camera),
    Spacecraft(Spacecraft),
    Asteroid(Asteroid),
}

impl From<Camera> for Entity {
    fn from(camera: Camera) -> Self {
        Entity::Camera(camera)
    }
}

impl From<Spacecraft> for Entity {
    fn from(spacecraft: Spacecraft) -> Self {
        Entity::Spacecraft(spacecraft)
    }
}

impl From<Asteroid> for Entity {
    fn from(asteroid: Asteroid) -> Self {
        Entity::Asteroid(asteroid)
    }
}

pub struct Entities {
    event_sender: Sender<Event>,
    entities: Mutex<Vec<Option<Entity>>>,
}

impl Entities {
    pub fn new(event_dispatcher: &Dispatcher<Event>) -> Arc<Entities> {
        let event_sender = event_dispatcher.create_sender();

        let entities = Entities {
            event_sender,
            entities: Default::default(),
        };

        Arc::new(entities)
    }

    pub fn create<E>(&self, entity: E) -> EntityId
    where
        E: Into<Entity>,
    {
        let mut entities = self.entities.lock().unwrap();

        let (index, should_insert) = entities
            .iter()
            .position(|slot| slot.is_none())
            .map(|index| (index, false))
            .unwrap_or_else(|| (entities.len(), true));

        let entity = entity.into();

        if should_insert {
            entities.push(Some(entity));
        } else {
            entities[index] = Some(entity);
        }

        self.event_sender.send(Event::EntityCreated(index));

        index
    }

    pub fn destroy(&self, entity_id: EntityId) {
        let mut entities = self.entities.lock().unwrap();

        if let Some(slot) = entities.get_mut(entity_id) {
            *slot = None;

            self.event_sender.send(Event::EntityDestroyed(entity_id));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::dispatch::{Dispatcher, Event};

    use super::{Entities, Spacecraft};

    #[test]
    fn entities_test() {
        let dispatcher = Dispatcher::new();
        let entities = Entities::new(&dispatcher);

        let entity_id = entities.create(Spacecraft::default());

        dispatcher.add_handler(move |event| match event {
            Event::EntityCreated(actual_entity_id) => assert_eq!(*actual_entity_id, entity_id),
            Event::EntityDestroyed(actual_entity_id) => assert_eq!(*actual_entity_id, entity_id),
            _ => panic!("event was not expected"),
        });
        dispatcher.dispatch();

        entities.destroy(entity_id);
    }
}
