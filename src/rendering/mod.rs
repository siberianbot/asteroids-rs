use std::sync::Arc;

use glam::Mat4;

use crate::game::entities::{Asteroid, Bullet, Camera, EntityId, Spacecraft};

mod backend;
mod models;
mod renderer;
mod shaders;

/// View data to use in rendering
pub struct View {
    matrix: Mat4,
}

impl From<&Camera> for View {
    fn from(value: &Camera) -> Self {
        Self {
            matrix: value.to_view_matrix(),
        }
    }
}

/// Model data to use in rendering
pub struct Model {
    matrix: Mat4,
}

impl From<&Spacecraft> for Model {
    fn from(value: &Spacecraft) -> Self {
        Self {
            matrix: value.transform.to_model_matrix(),
        }
    }
}

impl From<&Asteroid> for Model {
    fn from(value: &Asteroid) -> Self {
        Self {
            matrix: value.transform.to_model_matrix(),
        }
    }
}

impl From<&Bullet> for Model {
    fn from(value: &Bullet) -> Self {
        Self {
            matrix: value.transform.to_model_matrix(),
        }
    }
}

/// Renderer
pub struct Renderer {
    // TODO
}

impl Renderer {
    /// Creates new instance of [Renderer]
    pub fn new() -> Arc<Renderer> {
        todo!()
    }

    /// Sets entity to be used as view data source
    pub fn set_view(&self, entity_id: Option<EntityId>) {
        todo!()
    }

    /// Dispatches view data to renderer
    pub fn dispatch_view(&self, entity_id: EntityId, view: View) {
        todo!()
    }

    /// Dispatches model data to renderer
    pub fn dispatch_model(&self, entity_id: EntityId, model: Model) {
        todo!()
    }
}
