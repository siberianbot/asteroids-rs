use std::sync::{Arc, atomic::Ordering};

use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo,
        allocator::CommandBufferAllocator,
    },
    format::ClearValue,
    render_pass::{AttachmentLoadOp, AttachmentStoreOp},
};

use crate::worker::Worker;

use super::backend::Backend;

struct Inner {
    backend: Arc<Backend>,
    command_buffer_allocator: Arc<dyn CommandBufferAllocator>,
}

impl Inner {
    fn new(backend: Arc<Backend>) -> Inner {
        let inner = Inner {
            command_buffer_allocator: backend.create_command_buffer_allocator(),
            backend,
        };

        inner
    }

    fn render(&self) {
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
                    .expect("failed to begin rendering")
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
    pub fn new(backend: Arc<Backend>) -> Renderer {
        let renderer = Renderer {
            _worker: Worker::spawn("Renderer", move |alive| {
                let inner = Inner::new(backend);

                while alive.load(Ordering::Relaxed) {
                    inner.render();
                }
            }),
        };

        renderer
    }
}
