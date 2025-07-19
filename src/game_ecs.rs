use std::{
    collections::BTreeMap,
    ptr::NonNull,
    sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};

use crate::{
    dispatch::{Dispatcher, Event, Sender},
    game_entity::{Entity, EntityId},
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
    /// ECS update elapsed time
    pub elapsed: f32,
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

/// Trait for accessing entities collection behind the lock
pub trait EntitiesRead {
    /// Gets length of the entities collection
    fn len(&self) -> usize;

    /// Gets entity reference from the collection
    fn get(&self, entity_id: EntityId) -> Option<&Entity>;
}

/// Read-only lock over entities collection in [ECS]
pub struct EntitiesReadLock<'a> {
    entities: RwLockReadGuard<'a, Vec<Option<Entity>>>,
}

impl<'a> EntitiesReadLock<'a> {
    /// Gets entity
    pub fn get(&self, entity_id: EntityId) -> Option<&Entity> {
        self.entities.get(entity_id).and_then(|slot| slot.as_ref())
    }

    /// Iterates over all entities
    pub fn iter(&'a self) -> EntityIter<'a, EntitiesReadLock<'a>> {
        EntityIter {
            lock: self,
            entity_id: 0,
        }
    }
}

impl<'a> EntitiesRead for EntitiesReadLock<'a> {
    fn len(&self) -> usize {
        self.entities.len()
    }

    fn get(&self, entity_id: EntityId) -> Option<&Entity> {
        self.entities.get(entity_id).and_then(|slot| slot.as_ref())
    }
}

/// Lock over entities collection in [ECS] with ability to modify data
pub struct EntitiesWriteLock<'a> {
    entities: RwLockWriteGuard<'a, Vec<Option<Entity>>>,
    event_sender: Sender<Event>,
}

impl<'a> EntitiesWriteLock<'a> {
    /// Gets entity
    pub fn get(&self, entity_id: EntityId) -> Option<&Entity> {
        self.entities.get(entity_id).and_then(|slot| slot.as_ref())
    }

    /// Iterates over all entities
    pub fn iter(&'a self) -> EntityIter<'a, EntitiesWriteLock<'a>> {
        EntityIter {
            lock: self,
            entity_id: 0,
        }
    }

    /// Creates new entity
    pub fn create<E>(&mut self, entity: E) -> EntityId
    where
        E: Into<Entity>,
    {
        let (entity_id, should_insert) = self
            .entities
            .iter()
            .position(|slot| slot.is_none())
            .map(|index| (index, false))
            .unwrap_or_else(|| (self.entities.len(), true));

        let entity = entity.into();

        if should_insert {
            self.entities.push(Some(entity));
        } else {
            self.entities[entity_id] = Some(entity);
        }

        self.event_sender.send(Event::EntityCreated(entity_id));

        entity_id
    }

    /// Modifies entity
    pub fn modify<V, R>(&mut self, entity_id: EntityId, visitor: V) -> Option<R>
    where
        V: Fn(&mut Entity) -> R,
    {
        self.entities
            .get_mut(entity_id)
            .and_then(|slot| slot.as_mut())
            .map(visitor)
    }

    /// Destroys entity
    pub fn destroy(&mut self, entity_id: EntityId) {
        if let Some(slot) = self.entities.get_mut(entity_id) {
            *slot = None;

            self.event_sender.send(Event::EntityDestroyed(entity_id));
        }
    }
}

impl<'a> EntitiesRead for EntitiesWriteLock<'a> {
    fn len(&self) -> usize {
        self.entities.len()
    }

    fn get(&self, entity_id: EntityId) -> Option<&Entity> {
        self.entities.get(entity_id).and_then(|slot| slot.as_ref())
    }
}

/// Iterator over entities in [ECS]
pub struct EntityIter<'a, L>
where
    L: EntitiesRead,
{
    lock: &'a L,
    entity_id: EntityId,
}

impl<'a, L> Iterator for EntityIter<'a, L>
where
    L: EntitiesRead,
{
    type Item = (EntityId, &'a Entity);

    fn next(&mut self) -> Option<Self::Item> {
        while self.entity_id < self.lock.len() {
            let entity_id = self.entity_id;

            let tuple = self.lock.get(entity_id).map(|entity| unsafe {
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

    /// Locks entities collection for reading
    pub fn read(&self) -> EntitiesReadLock {
        EntitiesReadLock {
            entities: self.entities.read().unwrap(),
        }
    }

    /// Locks entities collection for writing
    pub fn write(&self) -> EntitiesWriteLock {
        EntitiesWriteLock {
            entities: self.entities.write().unwrap(),
            event_sender: self.event_sender.clone(),
        }
    }
}

/// INTERNAL: ECS worker thread function
fn worker_func(ecs: &ECS, elapsed: f32) {
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
                elapsed,
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
            let elapsed = Instant::now().duration_since(last_update).as_secs_f32();

            worker_func(&ecs, elapsed);

            last_update = Instant::now();

            if elapsed < UPDATE_RATE {
                let duration = Duration::from_secs_f32(UPDATE_RATE - elapsed);

                thread::sleep(duration);
            }
        }
    })
}
