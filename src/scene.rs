use std::{
    collections::BTreeMap,
    marker::PhantomData,
    ptr::NonNull,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use glam::{Mat4, Vec3};

use crate::{
    assets, events,
    game::entities::{self, EntityId},
    handle,
};

/// Scene entity with world view data
pub struct ViewSceneEntity {
    /// View matrix
    pub matrix: Mat4,
}

impl From<&entities::Camera> for ViewSceneEntity {
    fn from(value: &entities::Camera) -> Self {
        Self {
            matrix: value.to_view_matrix(),
        }
    }
}

/// Scene entity with model data
pub struct ModelSceneEntity {
    /// Model matrix
    pub matrix: Mat4,
    /// Color
    pub color: Vec3,
    /// Mesh asset reference
    pub mesh: assets::AssetRef,
    /// Pipeline asset reference
    pub pipeline: assets::AssetRef,
}

impl From<&entities::Spacecraft> for ModelSceneEntity {
    fn from(value: &entities::Spacecraft) -> Self {
        Self {
            matrix: value.transform.to_model_matrix(),
            color: Vec3::new(0.1, 0.8, 0.1),
            mesh: value.render.mesh.clone(),
            pipeline: value.render.pipeline.clone(),
        }
    }
}

impl From<&entities::Asteroid> for ModelSceneEntity {
    fn from(value: &entities::Asteroid) -> Self {
        Self {
            matrix: value.transform.to_model_matrix(),
            color: Vec3::new(0.6, 0.6, 0.6),
            mesh: value.render.mesh.clone(),
            pipeline: value.render.pipeline.clone(),
        }
    }
}

impl From<&entities::Bullet> for ModelSceneEntity {
    fn from(value: &entities::Bullet) -> Self {
        Self {
            matrix: value.transform.to_model_matrix(),
            color: Vec3::new(1.0, 1.0, 1.0),
            mesh: value.render.mesh.clone(),
            pipeline: value.render.pipeline.clone(),
        }
    }
}

/// Scene entity
///
/// See content of next structures for specific details:
/// * [ViewSceneEntity]
/// * [ModelSceneEntity]
pub enum SceneEntity {
    View(ViewSceneEntity),
    Model(ModelSceneEntity),
}

impl From<ViewSceneEntity> for SceneEntity {
    fn from(value: ViewSceneEntity) -> Self {
        Self::View(value)
    }
}

impl From<ModelSceneEntity> for SceneEntity {
    fn from(value: ModelSceneEntity) -> Self {
        Self::Model(value)
    }
}

impl<'a> From<&'a SceneEntity> for Option<&'a ViewSceneEntity> {
    fn from(value: &'a SceneEntity) -> Self {
        match value {
            SceneEntity::View(view) => Some(view),
            SceneEntity::Model(_) => None,
        }
    }
}

impl<'a> From<&'a SceneEntity> for Option<&'a ModelSceneEntity> {
    fn from(value: &'a SceneEntity) -> Self {
        match value {
            SceneEntity::View(_) => None,
            SceneEntity::Model(model) => Some(model),
        }
    }
}

/// INTERNAL: [Scene] data store
#[derive(Default)]
struct Store {
    entities: RwLock<BTreeMap<EntityId, SceneEntity>>,
}

/// Read-only lock over [SceneEntity] collection
pub struct SceneEntityReadLock<'a, T> {
    lock: RwLockReadGuard<'a, BTreeMap<EntityId, SceneEntity>>,
    entity_id: EntityId,
    _pd: PhantomData<T>,
}

impl<'a, T> SceneEntityReadLock<'a, T>
where
    T: 'a,
    Option<&'a T>: From<&'a SceneEntity>,
{
    pub fn get(&'a self) -> Option<&'a T> {
        self.lock
            .get(&self.entity_id)
            .and_then(|entity| entity.into())
    }
}

/// Read-only iterator over all [SceneEntity] in [Store]
pub struct SceneEntityIter<'a> {
    lock: RwLockReadGuard<'a, BTreeMap<EntityId, SceneEntity>>,
    entity_id: EntityId,
    max_entity_id: EntityId,
}

impl<'a> Iterator for SceneEntityIter<'a> {
    type Item = (EntityId, &'a SceneEntity);

    fn next(&mut self) -> Option<Self::Item> {
        while self.entity_id <= self.max_entity_id {
            let entity_id = self.entity_id;
            self.entity_id += 1;

            let tuple = self.lock.get(&entity_id).map(|player| unsafe {
                // HACK this iterator have a lock to container, references are valid 'till iterator lifetime
                (entity_id, NonNull::from(player).as_ref())
            });

            if tuple.is_some() {
                return tuple;
            }
        }

        None
    }
}

/// A scene
pub struct Scene {
    store: Arc<Store>,
    _handler: handle::Handle,
}

impl Scene {
    /// Creates new instance of [Scene]
    pub fn new(events: &events::Events) -> Arc<Scene> {
        let store: Arc<Store> = Default::default();

        let scene = Scene {
            store: store.clone(),
            _handler: events.add_handler(move |event| match event {
                events::Event::EntityDestroyed(entity_id) => {
                    store.entities.write().unwrap().remove(entity_id);
                }

                _ => {}
            }),
        };

        Arc::new(scene)
    }

    /// Dispatches scene entity
    pub fn dispatch<E>(&self, entity_id: EntityId, entity: E)
    where
        E: Into<SceneEntity>,
    {
        let mut entities = self.store.entities.write().unwrap();

        entities.insert(entity_id, entity.into());
    }

    /// Gets locked [SceneEntity] type by its [EntityId]
    pub fn get<'a, E>(&self, entity_id: EntityId) -> SceneEntityReadLock<E>
    where
        E: 'a,
        Option<&'a E>: From<&'a SceneEntity>,
    {
        SceneEntityReadLock {
            lock: self.store.entities.read().unwrap(),
            entity_id,
            _pd: Default::default(),
        }
    }

    /// Returns iterator over stored [SceneEntity]
    pub fn iter(&self) -> SceneEntityIter {
        let lock = self.store.entities.read().unwrap();

        SceneEntityIter {
            entity_id: lock
                .first_key_value()
                .map(|(entity_id, _)| *entity_id)
                .unwrap_or_default(),

            max_entity_id: lock
                .last_key_value()
                .map(|(entity_id, _)| *entity_id)
                .unwrap_or_default(),

            lock,
        }
    }
}
