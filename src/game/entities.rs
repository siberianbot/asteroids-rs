use std::{
    f32::consts::PI,
    ops::RangeInclusive,
    ptr::NonNull,
    sync::{Arc, Mutex, MutexGuard},
};

use glam::{Vec2, vec2};

use crate::{
    dispatch::{Dispatcher, Event, Sender},
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

impl Entity {
    pub fn as_camera(&self) -> Option<&Camera> {
        match self {
            Entity::Camera(camera) => Some(camera),
            _ => None,
        }
    }

    pub fn to_camera(&self) -> &Camera {
        match self {
            Entity::Camera(camera) => camera,
            _ => panic!("entity is not a camera"),
        }
    }

    pub fn to_camera_mut(&mut self) -> &mut Camera {
        match self {
            Entity::Camera(camera) => camera,
            _ => panic!("entity is not a camera"),
        }
    }

    pub fn as_spacecraft(&self) -> Option<&Spacecraft> {
        match self {
            Entity::Spacecraft(spacecraft) => Some(spacecraft),
            _ => None,
        }
    }

    pub fn to_spacecraft(&self) -> &Spacecraft {
        match self {
            Entity::Spacecraft(spacecraft) => spacecraft,
            _ => panic!("entity is not a spacecraft"),
        }
    }

    pub fn to_spacecraft_mut(&mut self) -> &mut Spacecraft {
        match self {
            Entity::Spacecraft(spacecraft) => spacecraft,
            _ => panic!("entity is not a spacecraft"),
        }
    }

    pub fn as_asteroid(&self) -> Option<&Asteroid> {
        match self {
            Entity::Asteroid(asteroid) => Some(asteroid),
            _ => None,
        }
    }

    pub fn to_asteroid(&self) -> &Asteroid {
        match self {
            Entity::Asteroid(asteroid) => asteroid,
            _ => panic!("entity is not a asteroid"),
        }
    }

    pub fn to_asteroid_mut(&mut self) -> &mut Asteroid {
        match self {
            Entity::Asteroid(asteroid) => asteroid,
            _ => panic!("entity is not a asteroid"),
        }
    }
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

pub struct UpdateContext<'a> {
    delta: f32,
    entities: &'a mut Vec<Option<Entity>>,
    current_entity_id: EntityId,
}

impl UpdateContext<'_> {
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
}

pub struct UpdateFunc(Box<dyn Fn(UpdateContext)>);

unsafe impl Send for UpdateFunc {}

impl<F> From<F> for UpdateFunc
where
    F: Fn(UpdateContext) + 'static,
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

pub struct Entities {
    event_sender: Sender<Event>,
    entities: Mutex<Vec<Option<Entity>>>,
    update_funcs: Mutex<Vec<UpdateFunc>>,
}

impl Entities {
    pub fn new<F, I>(event_dispatcher: &Dispatcher<Event>, update_funcs: I) -> Arc<Entities>
    where
        F: Into<UpdateFunc>,
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

    pub fn update(&self, delta: f32) {
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
                    current_entity_id: entity_id,
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
    use crate::dispatch::{Dispatcher, Event};

    use super::{Entities, Entity, Spacecraft, UpdateContext};

    #[test]
    fn entities_test() {
        const DELTA: f32 = 0.42;

        let dispatcher = Dispatcher::new();
        let entities = Entities::new(
            &dispatcher,
            [|context: UpdateContext| {
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

        entities.update(DELTA);

        entities.destroy(entity_id);
        dispatcher.dispatch();
    }
}
