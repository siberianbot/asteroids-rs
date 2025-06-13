use std::sync::{Arc, atomic::Ordering};

use vulkano::{
    buffer::{BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo,
        allocator::CommandBufferAllocator,
    },
    format::ClearValue,
    pipeline::{GraphicsPipeline, graphics::viewport::Viewport},
    render_pass::{AttachmentLoadOp, AttachmentStoreOp},
};

use crate::{
    dispatch::{Dispatcher, Event},
    game::{Game, entities::EntityId},
    rendering::{
        backend::{ShaderFactory, ShaderStage},
        shaders::{entity_fs, entity_vs},
    },
    worker::Worker,
};

use super::{backend::Backend, shaders::Entity};

struct Inner {
    game: Arc<Game>,
    backend: Arc<Backend>,
    entity_pipeline: Arc<GraphicsPipeline>,
    entity_buffer: Subbuffer<[Entity]>,
    command_buffer_allocator: Arc<dyn CommandBufferAllocator>,
}

impl Inner {
    fn new(game: Arc<Game>, backend: Arc<Backend>) -> Arc<Inner> {
        let inner = Inner {
            game,
            command_buffer_allocator: backend.create_command_buffer_allocator(),
            entity_pipeline: backend.create_pipeline([
                (ShaderStage::Vertex, entity_vs::load as ShaderFactory),
                (ShaderStage::Fragment, entity_fs::load as ShaderFactory),
            ]),
            entity_buffer: backend.create_buffer(1024, BufferUsage::UNIFORM_BUFFER),
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
                .end_rendering()
                .expect("failed to end rendering");

            let command_buffer = command_buffer_builder
                .build()
                .expect("failed to build command buffer");

            frame.submit(command_buffer);
        }
    }

    fn dispatch_entity_created(&self, entity_id: EntityId) {
        // TODO
    }

    fn dispatch_entity_destroyed(&self, entity_id: EntityId) {
        // TODO
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
