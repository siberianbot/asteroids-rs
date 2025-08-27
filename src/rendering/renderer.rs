use std::{
    collections::BTreeMap,
    f32::consts::PI,
    sync::{Arc, Mutex, RwLock},
};

use glam::{Mat4, Vec3};

use crate::{
    assets, events,
    game::entities::{Asteroid, Bullet, Camera, EntityId, Spacecraft},
    handle,
    rendering::{
        backend, buffer,
        commands::{self, CommandListSubmit},
        descriptors, frame, physical_device, pipeline,
    },
    workers,
};

mod vk {
    pub use vulkano::{
        command_buffer::{RenderingAttachmentInfo, RenderingInfo},
        descriptor_set::{DescriptorSet, WriteDescriptorSet},
        format::ClearValue,
        pipeline::graphics::viewport::Viewport,
        render_pass::{AttachmentLoadOp, AttachmentStoreOp},
    };
}

/// View data to use in rendering
pub struct ViewRenderData {
    matrix: Mat4,
}

impl From<&Camera> for ViewRenderData {
    fn from(value: &Camera) -> Self {
        Self {
            matrix: value.to_view_matrix(),
        }
    }
}

/// Model data to use in rendering
pub struct ModelRenderData {
    matrix: Mat4,
    color: Vec3,
    mesh: assets::AssetRef,
    pipeline: assets::AssetRef,
}

impl From<&Spacecraft> for ModelRenderData {
    fn from(value: &Spacecraft) -> Self {
        Self {
            matrix: value.transform.to_model_matrix(),
            color: Vec3::new(0.1, 0.8, 0.1),
            mesh: value.render.mesh.clone(),
            pipeline: value.render.pipeline.clone(),
        }
    }
}

impl From<&Asteroid> for ModelRenderData {
    fn from(value: &Asteroid) -> Self {
        Self {
            matrix: value.transform.to_model_matrix(),
            color: Vec3::new(0.6, 0.6, 0.6),
            mesh: value.render.mesh.clone(),
            pipeline: value.render.pipeline.clone(),
        }
    }
}

impl From<&Bullet> for ModelRenderData {
    fn from(value: &Bullet) -> Self {
        Self {
            matrix: value.transform.to_model_matrix(),
            color: Vec3::new(1.0, 1.0, 1.0),
            mesh: value.render.mesh.clone(),
            pipeline: value.render.pipeline.clone(),
        }
    }
}

/// Render data
///
/// See content of next structures for specific details:
/// * [ViewRenderData]
/// * [ModelRenderData]
pub enum RenderData {
    /// Variant of render data with [ViewRenderData]
    View(ViewRenderData),
    /// Variant of render data with [ModelRenderData]
    Model(ModelRenderData),
}

impl From<ViewRenderData> for RenderData {
    fn from(value: ViewRenderData) -> Self {
        Self::View(value)
    }
}

impl From<ModelRenderData> for RenderData {
    fn from(value: ModelRenderData) -> Self {
        Self::Model(value)
    }
}

/// INTERNAL: cache for model data
struct ModelCache {
    buffer: buffer::Buffer<assets::types::Model>,
    descriptor: Arc<vk::DescriptorSet>,
}

/// INTERNAL: some inner data store for [Renderer]
#[derive(Default)]
struct Store {
    view_entity_id: RwLock<Option<EntityId>>,
    render_data: Mutex<BTreeMap<EntityId, RenderData>>,
    model_cache: Mutex<BTreeMap<EntityId, ModelCache>>,
}

/// Renderer
pub struct Renderer {
    command_list_allocator: commands::CommandListAllocator,
    descriptor_allocator: descriptors::DescriptorAllocator,

    store: Arc<Store>,
    _handler: handle::Handle,

    backend: Arc<backend::Backend>,
    assets: Arc<assets::Assets>,
}

impl Renderer {
    /// Creates new instance of [Renderer]
    pub fn new(
        events: &events::Events,
        backend: Arc<backend::Backend>,
        assets: Arc<assets::Assets>,
    ) -> Arc<Renderer> {
        let store: Arc<Store> = Default::default();

        let renderer = Renderer {
            command_list_allocator: commands::CommandListAllocatorFactory::create(backend.as_ref()),
            descriptor_allocator: descriptors::DescriptorAllocatorFactory::create(backend.as_ref()),

            store: store.clone(),
            _handler: events.add_handler(move |event| match event {
                events::Event::EntityDestroyed(entity_id) => {
                    store.render_data.lock().unwrap().remove(entity_id);
                    store.model_cache.lock().unwrap().remove(entity_id);
                }

                _ => {}
            }),

            backend,
            assets,
        };

        Arc::new(renderer)
    }

    /// Sets entity to be used as view data source
    pub fn set_view(&self, entity_id: Option<EntityId>) {
        let mut view_entity_id = self.store.view_entity_id.write().unwrap();

        *view_entity_id = entity_id;
    }

    /// Dispatches render data to renderer
    pub fn dispatch<RD>(&self, entity_id: EntityId, data: RD)
    where
        RD: Into<RenderData>,
    {
        let mut render_data = self.store.render_data.lock().unwrap();

        render_data.insert(entity_id, data.into());
    }

    fn render_view(&self, frame: &frame::Frame, command_list: &mut commands::CommandList) {
        struct Item {
            pipeline: pipeline::Pipeline,
            descriptor: Arc<vk::DescriptorSet>,
            vertex: buffer::Buffer<assets::types::Vertex>,
            index: buffer::Buffer<u32>,
        }

        let render_data = self.store.render_data.lock().unwrap();
        let mut model_cache = self.store.model_cache.lock().unwrap();

        let camera_matrix = self
            .store
            .view_entity_id
            .read()
            .unwrap()
            .as_ref()
            .and_then(|view_entity_id| render_data.get(view_entity_id))
            .and_then(|data| match data {
                RenderData::View(view) => Some(view.matrix),
                _ => None,
            })
            .map(|view_matrix| {
                let [w, h] = frame.swapchain.extent;
                let aspect = w / h;
                let mut projection = Mat4::perspective_infinite_lh(PI / 2.0, aspect, 0.001);

                projection.col_mut(1)[1] *= -1.0;

                projection * view_matrix
            });

        if let None = camera_matrix {
            return;
        }

        let camera_matrix = camera_matrix.unwrap();

        let items = render_data
            .iter()
            .filter_map(|(entity_id, data)| match data {
                RenderData::Model(model) => Some((*entity_id, model)),
                _ => None,
            })
            .filter_map(|(entity_id, model)| {
                let pipeline = match self
                    .assets
                    .get(&model.pipeline)
                    .and_then(|asset| asset.as_pipeline().map(|asset| asset.pipeline.clone()))
                {
                    Some(pipeline) => pipeline,
                    None => return None,
                };

                let (vertex, index) = match self.assets.get(&model.mesh).and_then(|asset| {
                    asset
                        .as_mesh()
                        .map(|asset| (asset.vertex.clone(), asset.index.clone()))
                }) {
                    Some(mesh) => mesh,
                    None => return None,
                };

                let model_cache = model_cache
                    .entry(entity_id)
                    .and_modify(|model_cache| {
                        let mut buffer = model_cache.buffer.write();
                        let buffer_model = buffer.get_mut(0).unwrap();

                        *buffer_model = assets::types::Model {
                            color: model.color,
                            matrix: camera_matrix * model.matrix,
                        };
                    })
                    .or_insert_with(|| {
                        let buffer = buffer::BufferFactory::create(
                            self.backend.as_ref(),
                            buffer::BufferDef {
                                usage: buffer::BufferUsage::Uniform,
                                data: buffer::BufferData::Value(assets::types::Model {
                                    color: model.color,
                                    matrix: camera_matrix * model.matrix,
                                }),
                            },
                        );

                        let descriptor = self.descriptor_allocator.allocate(
                            &pipeline,
                            0,
                            [vk::WriteDescriptorSet::buffer(0, buffer.handle.clone())],
                            [],
                        );

                        ModelCache { buffer, descriptor }
                    });

                let item = Item {
                    pipeline,
                    descriptor: model_cache.descriptor.clone(),
                    vertex,
                    index,
                };

                Some(item)
            });

        for item in items {
            command_list.bind_pipeline(&item.pipeline);
            command_list.bind_vertex_buffer(&item.vertex);
            command_list.bind_index_buffer(&item.index);
            command_list.bind_descriptors(&item.pipeline, [item.descriptor]);
            command_list.draw(item.index.len(), 1);
        }
    }
}

/// INTERNAL: Renderer worker thread function
fn worker_func(renderer: &Renderer) {
    if let Some(frame) = frame::FrameFactory::try_acquire(renderer.backend.as_ref()) {
        let mut command_list = renderer.command_list_allocator.new_list(
            physical_device::QueueFamilyType::Graphics,
            commands::CommandListUsage::Multiple,
        );

        let image_view = frame
            .swapchain
            .image_views
            .get(frame.image_index as usize)
            .cloned()
            .expect("invalid image index");

        command_list.begin_rendering(vk::RenderingInfo {
            color_attachments: vec![Some(vk::RenderingAttachmentInfo {
                load_op: vk::AttachmentLoadOp::Clear,
                store_op: vk::AttachmentStoreOp::Store,
                clear_value: Some(vk::ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                ..vk::RenderingAttachmentInfo::image_view(image_view)
            })],
            ..Default::default()
        });

        command_list.set_viewports([vk::Viewport {
            offset: [0.0, 0.0],
            extent: frame.swapchain.extent,
            ..Default::default()
        }]);

        renderer.render_view(&frame, &mut command_list);

        command_list.end_rendering();

        frame.submit(command_list);
    }
}

/// Spawns renderer worker thread
pub fn spawn_worker(workers: &workers::Workers, renderer: Arc<Renderer>) -> handle::Handle {
    workers.spawn("Renderer", move |token| {
        while !token.is_cancelled() {
            worker_func(&renderer);
        }
    })
}
