use std::{
    f32::consts::PI,
    ops::RangeInclusive,
    ptr::NonNull,
    sync::{Arc, Mutex, MutexGuard},
};

use bitflags::bitflags;
use glam::{Vec2, vec2};

use crate::{
    dispatch::{Command, Dispatcher, Event, Sender},
    entity::Entity,
    rendering::shaders::Vertex,
};

pub type EntityId = usize;

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

pub const ASTEROID_SEGMENTS: usize = 8;
pub const ASTEROID_SEGMENT_RANGE: RangeInclusive<f32> = 0.75..=1.0;
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

pub const ASTEROID_SIZE_RANGE: RangeInclusive<u32> = 1..=4;
pub const ASTEROID_VELOCITY_RANGE: RangeInclusive<f32> = 0.25..=3.0;
pub const ASTEROID_ROTATION_VELOCITY_RANGE: RangeInclusive<f32> = 0.25..=2.0;

pub const BULLET_VERTICES: [Vertex; 1] = [Vertex {
    position: Vec2::new(0.0, 0.0),
}];
pub const BULLET_INDICES: [u32; 1] = [0];

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct PlayerAction : u32{
        const ACCELERATE = 1 << 0;
        const DECELERATE = 1 << 1;
        const INCLINE_LEFT = 1 << 2;
        const INCLINE_RIGHT = 1 << 3;
        const FIRE = 1 << 4;
    }
}

pub const CAMERA_INITIAL_DISTANCE: f32 = 4.0;
pub const CAMERA_MIN_DISTANCE: f32 = 1.0;
pub const CAMERA_MAX_DISTANCE: f32 = 32.0;
pub const CAMERA_DISTANCE_MULTIPLIER: f32 = 2.0;

pub struct UpdateContext<'a, T> {
    delta: f32,
    entities: &'a mut Vec<Option<Entity>>,
    event_sender: Sender<Event>,
    current_entity_id: EntityId,
    data: &'a T,
}

impl<T> UpdateContext<'_, T> {
    pub fn delta(&self) -> f32 {
        self.delta
    }

    pub fn current_entity(&self) -> &Entity {
        self.entities
            .get(self.current_entity_id)
            .and_then(|slot| slot.as_ref())
            .expect("invalid current entity")
    }

    pub fn get_entity(&self, entity_id: EntityId) -> Option<&Entity> {
        self.entities.get(entity_id).and_then(|slot| slot.as_ref())
    }

    pub fn modify<F>(self, func: F)
    where
        F: FnOnce(&mut Entity),
    {
        let entity = self
            .entities
            .get_mut(self.current_entity_id)
            .and_then(|slot| slot.as_mut())
            .expect("invalid current entity");

        func(entity);
    }

    pub fn destroy(self) {
        if let Some(slot) = self.entities.get_mut(self.current_entity_id) {
            *slot = None;

            self.event_sender
                .send(Event::EntityDestroyed(self.current_entity_id));
        }
    }

    pub fn data(&self) -> &T {
        self.data
    }
}

pub struct UpdateFunc<T>(Box<dyn Fn(UpdateContext<T>)>);

unsafe impl<T> Send for UpdateFunc<T> {}

impl<T, F> From<F> for UpdateFunc<T>
where
    F: Fn(UpdateContext<T>) + 'static,
{
    fn from(func: F) -> Self {
        let func = Box::new(func);

        UpdateFunc(func)
    }
}

struct Iter<'a> {
    entities: MutexGuard<'a, Vec<Option<Entity>>>,
    entity_id: EntityId,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (EntityId, &'a Entity);

    fn next(&mut self) -> Option<Self::Item> {
        while self.entity_id < self.entities.len() {
            let entity_id = self.entity_id;

            let tuple = self
                .entities
                .get(entity_id)
                .and_then(|slot| slot.as_ref())
                .map(|entity| unsafe {
                    // HACK this iterator locks entities container thus references are valid
                    (entity_id, NonNull::from(entity).as_ref())
                });

            self.entity_id += 1;

            if tuple.is_some() {
                return tuple;
            }
        }

        None
    }
}

pub struct Entities<T> {
    event_sender: Sender<Event>,
    entities: Mutex<Vec<Option<Entity>>>,
    update_funcs: Mutex<Vec<UpdateFunc<T>>>,
}

impl<T> Entities<T> {
    pub fn new<F, I>(event_dispatcher: &Dispatcher<Event>, update_funcs: I) -> Arc<Entities<T>>
    where
        F: Into<UpdateFunc<T>>,
        I: IntoIterator<Item = F>,
    {
        let event_sender = event_dispatcher.create_sender();
        let update_funcs = update_funcs.into_iter().map(|func| func.into()).collect();

        let entities = Entities {
            event_sender,
            entities: Default::default(),
            update_funcs: Mutex::new(update_funcs),
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

    pub fn update(&self, delta: f32, data: &T) {
        let mut entities = self.entities.lock().unwrap();
        let update_funcs = self.update_funcs.lock().unwrap();

        let entity_ids: Vec<EntityId> = entities
            .iter()
            .enumerate()
            .filter_map(|(entity_id, slot)| match slot.is_some() {
                true => Some(entity_id),
                false => None,
            })
            .collect();

        for entity_id in entity_ids {
            update_funcs.iter().for_each(|UpdateFunc(func)| {
                let context = UpdateContext {
                    delta,
                    entities: &mut entities,
                    event_sender: self.event_sender.clone(),
                    current_entity_id: entity_id,
                    data,
                };

                func(context);
            });
        }
    }

    pub fn destroy(&self, entity_id: EntityId) {
        let mut entities = self.entities.lock().unwrap();

        if let Some(slot) = entities.get_mut(entity_id) {
            *slot = None;

            self.event_sender.send(Event::EntityDestroyed(entity_id));
        }
    }

    pub fn visit<F, R>(&self, entity_id: EntityId, func: F) -> Option<R>
    where
        F: FnOnce(&Entity) -> R,
    {
        let entities = self.entities.lock().unwrap();

        entities
            .get(entity_id)
            .and_then(|slot| slot.as_ref())
            .map(func)
    }

    pub fn visit_mut<F, R>(&self, entity_id: EntityId, func: F) -> Option<R>
    where
        F: FnOnce(&mut Entity) -> R,
    {
        let mut entities = self.entities.lock().unwrap();

        entities
            .get_mut(entity_id)
            .and_then(|slot| slot.as_mut())
            .map(func)
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &Entity)> {
        Iter {
            entities: self.entities.lock().unwrap(),
            entity_id: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        dispatch::{Dispatcher, Event},
        entity::Spacecraft,
    };

    use super::{Entities, Entity, UpdateContext};

    #[test]
    fn entities_test() {
        const DELTA: f32 = 0.42;

        let dispatcher = Dispatcher::new();
        let entities = Entities::new(
            &dispatcher,
            [|context: UpdateContext<()>| {
                assert_eq!(context.delta(), DELTA);

                match context.current_entity() {
                    Entity::Spacecraft(_) => {}
                    _ => panic!("entity is not spacecraft"),
                };
            }],
        );

        let entity_id = entities.create(Spacecraft::default());

        dispatcher.add_handler(move |event| match event {
            Event::EntityCreated(actual_entity_id) => assert_eq!(*actual_entity_id, entity_id),
            Event::EntityDestroyed(actual_entity_id) => assert_eq!(*actual_entity_id, entity_id),
            _ => panic!("event was not expected"),
        });
        dispatcher.dispatch();

        entities.update(DELTA, &());

        entities.destroy(entity_id);
        dispatcher.dispatch();
    }
}
