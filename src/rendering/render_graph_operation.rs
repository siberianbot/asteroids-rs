use std::{
    collections::BTreeMap,
    f32::consts::PI,
    sync::{Arc, Mutex},
};

use glam::Mat4;

use crate::{
    assets, events,
    game::entities::EntityId,
    handle,
    rendering::{
        backend, buffer, descriptors,
        render_graph::{self, Arg},
    },
    scene,
};

mod vk {
    pub use vulkano::{
        descriptor_set::{DescriptorSet, WriteDescriptorSet},
        pipeline::graphics::viewport::Viewport,
    };
}

/// INTERNAL: cached model data
struct CachedModel {
    buffer: buffer::Buffer<assets::types::Model>,
    descriptor: Arc<vk::DescriptorSet>,
}

/// State for [scene_rendering_operation]
pub struct SceneRenderingOperationState {
    descriptor_allocator: descriptors::DescriptorAllocator,

    cached_models: Arc<Mutex<BTreeMap<EntityId, CachedModel>>>,

    assets: Arc<assets::Assets>,
    scene: Arc<scene::Scene>,
    backend: Arc<backend::Backend>,

    _handler: handle::Handle,
}

impl SceneRenderingOperationState {
    /// Creates new instance of [SceneRenderingOperationState]
    pub fn new(
        events: &events::Events,
        backend: Arc<backend::Backend>,
        assets: Arc<assets::Assets>,
        scene: Arc<scene::Scene>,
    ) -> SceneRenderingOperationState {
        let cached_models: Arc<Mutex<BTreeMap<EntityId, CachedModel>>> = Default::default();

        SceneRenderingOperationState {
            descriptor_allocator: descriptors::DescriptorAllocatorFactory::create(backend.as_ref()),

            cached_models: cached_models.clone(),

            assets,
            scene,
            backend,

            _handler: events.add_handler(move |event| match event {
                events::Event::EntityDestroyed(entity_id) => {
                    cached_models.lock().unwrap().remove(entity_id);
                }

                _ => {}
            }),
        }
    }
}

/// Scene rendering operation: renders entire scene
pub fn scene_rendering_operation(
    state: &SceneRenderingOperationState,
    context: render_graph::OperationContext,
) {
    let [w, h] = context
        .attachments
        .color
        .get(0)
        .expect("there is no color target")
        .extent;

    let view_entity_id = context
        .args
        .get("view_entity_id")
        .and_then(|arg| match arg {
            Arg::EntityId(entity_id) => Some(*entity_id),
        })
        .expect("there is no view entity ID provided");

    context.command_list.set_viewports([vk::Viewport {
        offset: [0.0, 0.0],
        extent: [w, h],
        ..Default::default()
    }]);

    let projection_view_matrix = state
        .scene
        .get::<scene::ViewSceneEntity>(view_entity_id)
        .get()
        .map(|view| {
            let aspect = w / h;
            let mut projection = Mat4::perspective_infinite_lh(PI / 2.0, aspect, 0.001);
            projection.col_mut(1)[1] *= -1.0;

            projection * view.matrix
        });

    if let None = projection_view_matrix {
        return;
    }

    let projection_view_matrix = projection_view_matrix.unwrap();
    let mut cached_models = state.cached_models.lock().unwrap();

    let items = state
        .scene
        .iter()
        .filter_map(|(entity_id, entity)| match entity {
            scene::SceneEntity::Model(model) => Some((entity_id, model)),
            _ => None,
        })
        .filter_map(|(entity_id, model)| {
            let pipeline = match state
                .assets
                .get(&model.pipeline)
                .and_then(|asset| asset.as_pipeline().map(|asset| asset.pipeline.clone()))
            {
                Some(pipeline) => pipeline,
                None => return None,
            };

            let (vertex, index) = match state.assets.get(&model.mesh).and_then(|asset| {
                asset
                    .as_mesh()
                    .map(|asset| (asset.vertex.clone(), asset.index.clone()))
            }) {
                Some(mesh) => mesh,
                None => return None,
            };

            let model_cache = cached_models
                .entry(entity_id)
                .and_modify(|model_cache| {
                    let mut buffer = model_cache.buffer.write();
                    let buffer_model = buffer.get_mut(0).unwrap();

                    *buffer_model = assets::types::Model {
                        color: model.color,
                        matrix: projection_view_matrix * model.matrix,
                    };
                })
                .or_insert_with(|| {
                    let buffer = buffer::BufferFactory::create(
                        state.backend.as_ref(),
                        buffer::BufferDef {
                            usage: buffer::BufferUsage::Uniform,
                            data: buffer::BufferData::Value(assets::types::Model {
                                color: model.color,
                                matrix: projection_view_matrix * model.matrix,
                            }),
                        },
                    );

                    let descriptor = state.descriptor_allocator.allocate(
                        &pipeline,
                        0,
                        [vk::WriteDescriptorSet::buffer(0, buffer.handle.clone())],
                        [],
                    );

                    CachedModel { buffer, descriptor }
                });

            let item = (pipeline, model_cache.descriptor.clone(), vertex, index);

            Some(item)
        });

    for (pipeline, descriptor, vertex, index) in items {
        context.command_list.bind_pipeline(&pipeline);
        context.command_list.bind_vertex_buffer(&vertex);
        context.command_list.bind_index_buffer(&index);
        context
            .command_list
            .bind_descriptors(&pipeline, [descriptor]);
        context.command_list.draw(index.len(), 1);
    }
}
