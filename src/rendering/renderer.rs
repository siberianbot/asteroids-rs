use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

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
    game::Game,
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
    recreate_swapchain: AtomicBool,
    recreate_swapchain_size: Mutex<[u32; 2]>,
    entity_pipeline: Arc<GraphicsPipeline>,
    entity_buffer: Subbuffer<[Entity]>,
    command_buffer_allocator: Arc<dyn CommandBufferAllocator>,
}

impl Inner {
    fn new(game: Arc<Game>, backend: Arc<Backend>) -> Arc<Inner> {
        let inner = Inner {
            game,
            recreate_swapchain: AtomicBool::new(false),
            recreate_swapchain_size: Mutex::new([0, 0]),
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
        if self.recreate_swapchain.load(Ordering::Acquire) {
            let size = self.recreate_swapchain_size.lock().unwrap();
            self.backend.recreate_swapchain(Some(*size));

            self.recreate_swapchain.store(false, Ordering::Release);
        }

        match self.backend.acquire_frame() {
            Some(frame) => {
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

            None => {
                self.backend.recreate_swapchain(None);
            }
        }
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

            event_dispatcher.add_handler(move |event| {
                if let Event::WindowResized(size) = event {
                    *inner.recreate_swapchain_size.lock().unwrap() = *size;

                    inner.recreate_swapchain.store(true, Ordering::Relaxed);
                }
            });
        }

        let renderer = {
            let inner = inner.clone();

            Renderer {
                _worker: Worker::spawn("Renderer", move |alive| {
                    while alive.load(Ordering::Relaxed) {
                        inner.render();
                    }
                }),
            }
        };

        renderer
    }
}
