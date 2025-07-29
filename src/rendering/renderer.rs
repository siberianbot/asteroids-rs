use std::{
    collections::BTreeMap,
    f32::consts::PI,
    sync::{Arc, Mutex, RwLock, atomic::Ordering},
};

use glam::{Mat4, Vec3};
use vulkano::{
    buffer::{BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer,
        RenderingAttachmentInfo, RenderingInfo, allocator::CommandBufferAllocator,
    },
    descriptor_set::{DescriptorSet, WriteDescriptorSet, allocator::DescriptorSetAllocator},
    format::ClearValue,
    pipeline::{
        GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout, graphics::viewport::Viewport,
    },
    render_pass::{AttachmentLoadOp, AttachmentStoreOp},
};

use crate::{
    assets::{AssetRef, Assets, types::Vertex},
    dispatch::{Dispatcher, Event},
    game::entities::{Asteroid, Bullet, Camera, EntityId, Spacecraft},
    rendering::{
        backend::{Backend, Frame},
        shaders,
    },
    worker::Worker,
};

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
    mesh: AssetRef,
    pipeline: AssetRef,
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
    buffer: Subbuffer<shaders::Model>,
    descriptor_set: Arc<DescriptorSet>,
}

/// Renderer
pub struct Renderer {
    command_buffer_allocator: Arc<dyn CommandBufferAllocator>,
    descriptor_set_allocator: Arc<dyn DescriptorSetAllocator>,

    view_entity_id: RwLock<Option<EntityId>>,
    render_data: Mutex<BTreeMap<EntityId, RenderData>>,
    model_cache: Mutex<BTreeMap<EntityId, ModelCache>>,

    backend: Arc<Backend>,
    assets: Arc<Assets>,
}

impl Renderer {
    /// Creates new instance of [Renderer]
    pub fn new(
        events: &Dispatcher<Event>,
        backend: Arc<Backend>,
        assets: Arc<Assets>,
    ) -> Arc<Renderer> {
        let renderer = Renderer {
            command_buffer_allocator: backend.create_command_buffer_allocator(),
            descriptor_set_allocator: backend.create_descriptor_set_allocator(),

            view_entity_id: Default::default(),
            render_data: Default::default(),
            model_cache: Default::default(),

            backend,
            assets,
        };

        let renderer = Arc::new(renderer);

        {
            let renderer = renderer.clone();

            events.add_handler(move |event| match event {
                Event::EntityCreated(entity_id) => {
                    renderer.render_data.lock().unwrap().remove(entity_id);
                    renderer.model_cache.lock().unwrap().remove(entity_id);
                }

                _ => {}
            });
        }

        renderer
    }

    /// Sets entity to be used as view data source
    pub fn set_view(&self, entity_id: Option<EntityId>) {
        let mut view_entity_id = self.view_entity_id.write().unwrap();

        *view_entity_id = entity_id;
    }

    /// Dispatches render data to renderer
    pub fn dispatch<RD>(&self, entity_id: EntityId, data: RD)
    where
        RD: Into<RenderData>,
    {
        let mut render_data = self.render_data.lock().unwrap();

        render_data.insert(entity_id, data.into());
    }

    fn render_view(
        &self,
        frame: &Frame,
        command_buffer_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        struct Item {
            pipeline: Arc<GraphicsPipeline>,
            pipeline_layout: Arc<PipelineLayout>,
            pipeline_descriptor_set: Arc<DescriptorSet>,
            vertex: Subbuffer<[Vertex]>,
            index: Subbuffer<[u32]>,
            index_len: u64,
        }

        let render_data = self.render_data.lock().unwrap();
        let mut model_cache = self.model_cache.lock().unwrap();

        let camera_matrix = self
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
                let mut projection = Mat4::perspective_infinite_lh(PI / 2.0, frame.aspect(), 0.001);

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

                let model_cache = model_cache.entry(entity_id).or_insert_with(|| {
                    let layout = pipeline
                        .layout()
                        .set_layouts()
                        .get(0)
                        .cloned()
                        .expect("invalid descriptor set layout");

                    let buffer = self.backend.create_buffer(BufferUsage::UNIFORM_BUFFER);

                    let descriptor_set = DescriptorSet::new(
                        self.descriptor_set_allocator.clone(),
                        layout,
                        [WriteDescriptorSet::buffer(0, buffer.clone())],
                        [],
                    )
                    .expect("failed to create descriptor set for entity");

                    ModelCache {
                        buffer,
                        descriptor_set,
                    }
                });

                let mut buffer = model_cache
                    .buffer
                    .write()
                    .expect("failed to update model buffer");

                buffer.color = model.color;
                buffer.matrix = camera_matrix * model.matrix;

                let item = Item {
                    pipeline_layout: pipeline.layout().clone(),
                    pipeline_descriptor_set: model_cache.descriptor_set.clone(),
                    index_len: index.len(),
                    pipeline,
                    vertex,
                    index,
                };

                Some(item)
            });

        for item in items {
            command_buffer_builder
                .bind_pipeline_graphics(item.pipeline)
                .expect("failed to bind entity pipeline");

            command_buffer_builder
                .bind_vertex_buffers(0, item.vertex)
                .expect("failed to bind vertex buffer");

            command_buffer_builder
                .bind_index_buffer(item.index)
                .expect("failed to bind index buffer");

            command_buffer_builder
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    item.pipeline_layout,
                    0,
                    vec![item.pipeline_descriptor_set],
                )
                .expect("failed to bind descriptor set");

            unsafe {
                command_buffer_builder
                    .draw_indexed(item.index_len as u32, 1, 0, 0, 0)
                    .expect("failed to draw entity");
            }
        }
    }
}

/// INTERNAL: Renderer worker thread function
fn worker_func(renderer: &Renderer) {
    if let Some(frame) = renderer.backend.acquire_frame() {
        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            renderer.command_buffer_allocator.clone(),
            renderer.backend.graphics_queue_family_index(),
            CommandBufferUsage::MultipleSubmit,
        )
        .expect("failed to create command buffer builder");

        command_buffer_builder
            .begin_rendering(RenderingInfo {
                color_attachments: vec![Some(RenderingAttachmentInfo {
                    load_op: AttachmentLoadOp::Clear,
                    store_op: AttachmentStoreOp::Store,
                    clear_value: Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),

                    ..RenderingAttachmentInfo::image_view(frame.image_view())
                })],
                ..Default::default()
            })
            .expect("failed to begin rendering");

        command_buffer_builder
            .set_viewport(
                0,
                vec![Viewport {
                    offset: [0.0, 0.0],
                    extent: frame.extent(),
                    ..Default::default()
                }]
                .into(),
            )
            .expect("failed to set viewport");

        renderer.render_view(&frame, &mut command_buffer_builder);

        command_buffer_builder
            .end_rendering()
            .expect("failed to end rendering");

        let command_buffer = command_buffer_builder
            .build()
            .expect("failed to build command buffer");

        frame.submit(command_buffer);
    }
}

/// Spawns renderer worker thread
pub fn spawn_worker(renderer: Arc<Renderer>) -> Worker {
    Worker::spawn("Renderer", move |alive| {
        while alive.load(Ordering::Relaxed) {
            worker_func(&renderer);
        }
    })
}
