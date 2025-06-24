use std::{
    collections::BTreeMap,
    f32::consts::PI,
    iter::once,
    sync::{Arc, Mutex, atomic::Ordering},
};

use glam::{Mat4, Quat, Vec2, Vec3};
use vulkano::{
    buffer::{BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo,
        allocator::CommandBufferAllocator,
    },
    descriptor_set::{DescriptorSet, WriteDescriptorSet, allocator::DescriptorSetAllocator},
    format::ClearValue,
    pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint, graphics::viewport::Viewport},
    render_pass::{AttachmentLoadOp, AttachmentStoreOp},
};

use crate::{
    dispatch::{Dispatcher, Event},
    game::{
        Game,
        entities::{self, ASTEROID_INDICES, EntityId, SPACECRAFT_INDICES, SPACECRAFT_VERTICES},
    },
    rendering::{
        backend::{ShaderFactory, ShaderStage},
        shaders::{Vertex, entity_fs, entity_vs},
    },
    worker::Worker,
};

use super::{backend::Backend, shaders::Entity};

struct RenderData {
    entity_buffer: Subbuffer<Entity>,
    entity_buffer_descriptor_set: Arc<DescriptorSet>,
    vertex_buffer: Subbuffer<[Vertex]>,
    index_buffer: Subbuffer<[u32]>,
}

struct Inner {
    game: Arc<Game>,
    backend: Arc<Backend>,
    entity_pipeline: Arc<GraphicsPipeline>,
    render_data: Mutex<BTreeMap<EntityId, RenderData>>,
    spacecraft_vertex_buffer: Subbuffer<[Vertex]>,
    spacecraft_index_buffer: Subbuffer<[u32]>,
    asteroid_index_buffer: Subbuffer<[u32]>,
    command_buffer_allocator: Arc<dyn CommandBufferAllocator>,
    descriptor_set_allocator: Arc<dyn DescriptorSetAllocator>,
}

impl Inner {
    fn new(game: Arc<Game>, backend: Arc<Backend>) -> Arc<Inner> {
        let inner = Inner {
            game,
            command_buffer_allocator: backend.create_command_buffer_allocator(),
            descriptor_set_allocator: backend.create_descriptor_set_allocator(),
            entity_pipeline: backend.create_pipeline([
                (ShaderStage::Vertex, entity_vs::load as ShaderFactory),
                (ShaderStage::Fragment, entity_fs::load as ShaderFactory),
            ]),
            render_data: Default::default(),
            spacecraft_vertex_buffer: backend
                .create_buffer_iter(BufferUsage::VERTEX_BUFFER, SPACECRAFT_VERTICES),
            spacecraft_index_buffer: backend
                .create_buffer_iter(BufferUsage::INDEX_BUFFER, SPACECRAFT_INDICES),
            asteroid_index_buffer: backend
                .create_buffer_iter(BufferUsage::INDEX_BUFFER, ASTEROID_INDICES),
            backend,
        };

        Arc::new(inner)
    }

    fn render(&self) {
        if let Some(frame) = self.backend.acquire_frame() {
            let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
                self.command_buffer_allocator.clone(),
                self.backend.graphics_queue_family_index(),
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

            command_buffer_builder
                .bind_pipeline_graphics(self.entity_pipeline.clone())
                .expect("failed to bind entities pipeline");

            let entities = self.game.entities();
            let mut render_data = self.render_data.lock().unwrap();

            let camera_matrix = entities
                .visit(self.game.camera_entity_id(), |entity| {
                    let camera = entity.to_camera();

                    let mut projection =
                        Mat4::perspective_infinite_lh(PI / 2.0, frame.aspect(), 0.001);
                    projection.col_mut(1)[1] *= -1.0;

                    let view = Mat4::look_at_lh(
                        Vec3::new(camera.position.x, camera.position.y, camera.distance),
                        Vec3::new(camera.position.x, camera.position.y, 0.0),
                        Vec3::new(0.0, 1.0, 0.0),
                    );

                    projection * view
                })
                .expect("there is not camera entity");

            entities
                .iter()
                .filter(|(_, entity)| !matches!(entity, entities::Entity::Camera(_)))
                .for_each(|(entity_id, entity)| {
                    let render_data = render_data.entry(entity_id).or_insert_with(|| {
                        self.create_render_data(entity)
                            .expect("entity is not renderable")
                    });

                    {
                        let mut entity_buffer = render_data.entity_buffer.write().unwrap();

                        let model = match entity {
                            entities::Entity::Spacecraft(spacecraft) => {
                                Mat4::from_scale_rotation_translation(
                                    Vec3::ONE,
                                    Quat::from_rotation_z(spacecraft.rotation),
                                    Vec3::new(spacecraft.position.x, spacecraft.position.y, 0.0),
                                )
                            }

                            entities::Entity::Asteroid(asteroid) => {
                                Mat4::from_scale_rotation_translation(
                                    Vec3::ONE,
                                    Quat::from_rotation_z(asteroid.rotation),
                                    Vec3::new(asteroid.position.x, asteroid.position.y, 0.0),
                                )
                            }

                            _ => unreachable!(),
                        };

                        entity_buffer.matrix = camera_matrix * model
                    }

                    command_buffer_builder
                        .bind_vertex_buffers(0, render_data.vertex_buffer.clone())
                        .expect("failed to bind vertex buffer");

                    command_buffer_builder
                        .bind_index_buffer(render_data.index_buffer.clone())
                        .expect("failed to bind index buffer");

                    command_buffer_builder
                        .bind_descriptor_sets(
                            PipelineBindPoint::Graphics,
                            self.entity_pipeline.layout().clone(),
                            0,
                            vec![render_data.entity_buffer_descriptor_set.clone()],
                        )
                        .expect("failed to bind descriptor set");

                    unsafe {
                        command_buffer_builder
                            .draw_indexed(render_data.index_buffer.len() as u32, 1, 0, 0, 0)
                            .expect("failed to draw entity");
                    }
                });

            command_buffer_builder
                .end_rendering()
                .expect("failed to end rendering");

            let command_buffer = command_buffer_builder
                .build()
                .expect("failed to build command buffer");

            frame.submit(command_buffer);
        }
    }

    fn create_render_data(&self, entity: &entities::Entity) -> Option<RenderData> {
        let entity_buffer: Subbuffer<Entity> =
            self.backend.create_buffer(BufferUsage::UNIFORM_BUFFER);
        let entity_buffer_descriptor_set = {
            let layout = self
                .entity_pipeline
                .layout()
                .set_layouts()
                .get(0)
                .cloned()
                .unwrap();

            DescriptorSet::new(
                self.descriptor_set_allocator.clone(),
                layout,
                [WriteDescriptorSet::buffer(0, entity_buffer.clone())],
                [],
            )
            .expect("failed to create descriptor set for entity")
        };

        let render_data = match entity {
            entities::Entity::Spacecraft(_) => {
                {
                    entity_buffer.write().unwrap().color = Vec3::new(0.1, 0.8, 0.1);
                }

                Some(RenderData {
                    entity_buffer,
                    entity_buffer_descriptor_set,
                    vertex_buffer: self.spacecraft_vertex_buffer.clone(),
                    index_buffer: self.spacecraft_index_buffer.clone(),
                })
            }

            entities::Entity::Asteroid(asteroid) => {
                {
                    entity_buffer.write().unwrap().color = Vec3::new(0.6, 0.6, 0.6);
                }

                let vertices = once(Vec2::ZERO)
                    .chain(asteroid.body.into_iter())
                    .map(|position| Vertex { position })
                    .collect::<Vec<_>>();

                Some(RenderData {
                    entity_buffer,
                    entity_buffer_descriptor_set,
                    vertex_buffer: self
                        .backend
                        .create_buffer_iter(BufferUsage::VERTEX_BUFFER, vertices),
                    index_buffer: self.asteroid_index_buffer.clone(),
                })
            }

            _ => None,
        };

        render_data
    }

    fn dispatch_entity_created(&self, entity_id: EntityId) {
        let entities = self.game.entities();

        let data = entities
            .visit(entity_id, |entity| self.create_render_data(entity))
            .expect("entity not found");

        if let Some(data) = data {
            self.render_data.lock().unwrap().insert(entity_id, data);
        }
    }

    fn dispatch_entity_destroyed(&self, entity_id: EntityId) {
        self.render_data.lock().unwrap().remove(&entity_id);
    }
}

pub struct Renderer {
    _worker: Worker,
}

impl Renderer {
    pub fn new(
        event_dispatcher: &Dispatcher<Event>,
        game: Arc<Game>,
        backend: Arc<Backend>,
    ) -> Renderer {
        let inner = Inner::new(game, backend);

        {
            let inner = inner.clone();

            event_dispatcher.add_handler(move |event| match event {
                Event::EntityCreated(entity_id) => inner.dispatch_entity_created(*entity_id),
                Event::EntityDestroyed(entity_id) => inner.dispatch_entity_destroyed(*entity_id),

                _ => {}
            });
        }

        let renderer = Renderer {
            _worker: Worker::spawn("Renderer", move |alive| {
                while alive.load(Ordering::Relaxed) {
                    inner.render();
                }
            }),
        };

        renderer
    }
}
