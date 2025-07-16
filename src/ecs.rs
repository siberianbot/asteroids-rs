use std::{
    collections::BTreeMap,
    ptr::NonNull,
    sync::{Arc, Mutex, RwLock, RwLockReadGuard, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};

use crate::{
    dispatch::{Dispatcher, Event, Sender},
    entity::{Entity, EntityId},
    worker::Worker,
};

/// INTERNAL: action over entity which ECS should perform, enqueued by system
enum Action {
    /// Modify entity
    Modify(EntityId, Box<dyn FnOnce(&mut Entity)>),
    /// Destroy entity
    Destroy(EntityId),
}

/// System invocation arguments
pub struct SystemArgs<'a> {
    /// ECS update delta time
    pub delta: f32,
    /// Current ID of entity
    pub entity_id: EntityId,
    /// Current entity
    pub entity: &'a Entity,

    entities: &'a Vec<Option<Entity>>,
    actions: &'a mut Vec<Action>,
}

impl<'a> SystemArgs<'a> {
    /// Gets entity by its ID
    pub fn get_entity(&self, entity_id: EntityId) -> Option<&Entity> {
        self.entities.get(entity_id).and_then(|slot| slot.as_ref())
    }

    /// Enqueues modification of the current entity
    pub fn modify<F>(self, func: F)
    where
        F: FnOnce(&mut Entity) + 'static,
    {
        let action = Action::Modify(self.entity_id, Box::new(func));

        self.actions.push(action);
    }

    /// Enqueues destruction of the current entity
    pub fn destroy(self) {
        let action = Action::Destroy(self.entity_id);

        self.actions.push(action);
    }
}

/// Trait of a system, which handles changes of single entity
pub trait System: Send + Sync {
    /// Invoke system
    fn invoke(&self, args: SystemArgs);
}

/// Stateless [System] implementation
pub struct StatelessSystem {
    delegate: Box<dyn Fn(SystemArgs)>,
}

impl System for StatelessSystem {
    fn invoke(&self, args: SystemArgs) {
        (self.delegate)(args)
    }
}

impl<F> From<F> for StatelessSystem
where
    F: Fn(SystemArgs) + 'static,
{
    fn from(value: F) -> Self {
        Self {
            delegate: Box::new(value),
        }
    }
}

unsafe impl Send for StatelessSystem {}

unsafe impl Sync for StatelessSystem {}

/// Stateful [System] implementation
pub struct StatefulSystem<S> {
    state: S,
    delegate: Box<dyn Fn(SystemArgs, &S)>,
}

impl<S> StatefulSystem<S> {
    /// Creates new instance of [StatefulSystem] with some predefined state
    pub fn new<F>(state: S, delegate: F) -> StatefulSystem<S>
    where
        F: Fn(SystemArgs, &S) + 'static,
    {
        StatefulSystem {
            state,
            delegate: Box::new(delegate),
        }
    }
}

impl<S> System for StatefulSystem<S>
where
    S: Send + Sync,
{
    fn invoke(&self, args: SystemArgs) {
        (self.delegate)(args, &self.state)
    }
}

impl<S, F> From<F> for StatefulSystem<S>
where
    S: Default,
    F: Fn(SystemArgs, &S) + 'static,
{
    fn from(value: F) -> Self {
        Self {
            state: Default::default(),
            delegate: Box::new(value),
        }
    }
}

unsafe impl<S> Send for StatefulSystem<S> where S: Send + Sync {}

unsafe impl<S> Sync for StatefulSystem<S> where S: Send + Sync {}

/// Iterator over entities in [ECS]
pub struct EntityIter<'a> {
    entities: RwLockReadGuard<'a, Vec<Option<Entity>>>,
    entity_id: EntityId,
}

impl<'a> Iterator for EntityIter<'a> {
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

/// Entity-Component-System infrastructure
pub struct ECS {
    event_sender: Sender<Event>,
    entities: RwLock<Vec<Option<Entity>>>,
    systems: Mutex<BTreeMap<String, Box<dyn System>>>,
}

impl ECS {
    /// Creates new instance of [ECS]
    pub fn new(event_dispatcher: &Dispatcher<Event>) -> Arc<ECS> {
        let ecs = ECS {
            event_sender: event_dispatcher.create_sender(),
            entities: Default::default(),
            systems: Default::default(),
        };

        Arc::new(ecs)
    }

    /// Adds system
    pub fn add_system<N, S>(&self, name: N, system: S)
    where
        N: Into<String>,
        S: System + 'static,
    {
        let mut systems = self.systems.lock().unwrap();

        systems.insert(name.into(), Box::new(system));
    }

    /// Removes system
    pub fn remove_system<N>(&self, name: N)
    where
        N: Into<String>,
    {
        let mut systems = self.systems.lock().unwrap();

        systems.remove(&name.into());
    }

    /// Creates new entity
    pub fn create_entity<E>(&self, entity: E) -> EntityId
    where
        E: Into<Entity>,
    {
        let mut entities = self.entities.write().unwrap();

        let (entity_id, should_insert) = entities
            .iter()
            .position(|slot| slot.is_none())
            .map(|index| (index, false))
            .unwrap_or_else(|| (entities.len(), true));

        let entity = entity.into();

        if should_insert {
            entities.push(Some(entity));
        } else {
            entities[entity_id] = Some(entity);
        }

        self.event_sender.send(Event::EntityCreated(entity_id));

        entity_id
    }

    /// Destroys entity
    pub fn destroy_entity(&self, entity_id: EntityId) {
        let mut entities = self.entities.write().unwrap();

        if let Some(slot) = entities.get_mut(entity_id) {
            *slot = None;

            self.event_sender.send(Event::EntityDestroyed(entity_id));
        }
    }

    /// Visits entity immutably
    pub fn visit_entity<V, R>(&self, entity_id: EntityId, visitor: V) -> Option<R>
    where
        V: Fn(&Entity) -> R,
    {
        let entities = self.entities.read().unwrap();

        entities
            .get(entity_id)
            .and_then(|slot| slot.as_ref())
            .map(visitor)
    }

    /// Visits entity and allows its mutation
    pub fn visit_entity_mut<V, R>(&self, entity_id: EntityId, visitor: V) -> Option<R>
    where
        V: Fn(&mut Entity) -> R,
    {
        let mut entities = self.entities.write().unwrap();

        entities
            .get_mut(entity_id)
            .and_then(|slot| slot.as_mut())
            .map(visitor)
    }

    /// Iterates over all available entities
    pub fn iter_entities(&self) -> EntityIter {
        EntityIter {
            entities: self.entities.read().unwrap(),
            entity_id: 0,
        }
    }
}

/// INTERNAL: ECS worker thread function
fn worker_func(ecs: &ECS, delta: f32) {
    let mut entities = ecs.entities.write().unwrap();
    let systems = ecs.systems.lock().unwrap();

    let mut actions = Vec::new();

    let iter = entities
        .iter()
        .enumerate()
        .filter_map(|(entity_id, slot)| slot.as_ref().map(|entity| (entity_id, entity)));

    for (entity_id, entity) in iter {
        for (_, system) in systems.iter() {
            let args = SystemArgs {
                delta,
                entity_id,
                entity,

                entities: &entities,
                actions: &mut actions,
            };

            system.invoke(args);
        }
    }

    for action in actions {
        match action {
            Action::Modify(entity_id, func) => {
                if let Some(entity) = entities.get_mut(entity_id).and_then(|slot| slot.as_mut()) {
                    func(entity);
                }
            }

            Action::Destroy(entity_id) => {
                if let Some(slot) = entities.get_mut(entity_id) {
                    *slot = None;

                    ecs.event_sender.send(Event::EntityDestroyed(entity_id));
                }
            }
        }
    }
}

/// Spawns ECS worker thread
pub fn spawn_worker(ecs: Arc<ECS>) -> Worker {
    Worker::spawn("ECS", move |alive| {
        const UPDATE_RATE: f32 = 1.0 / 120.0;

        let mut last_update = Instant::now();

        while alive.load(Ordering::Relaxed) {
            let delta = Instant::now().duration_since(last_update).as_secs_f32();

            worker_func(&ecs, delta);

            last_update = Instant::now();

            if delta < UPDATE_RATE {
                let duration = Duration::from_secs_f32(UPDATE_RATE - delta);

                thread::sleep(duration);
            }
        }
    })
}
