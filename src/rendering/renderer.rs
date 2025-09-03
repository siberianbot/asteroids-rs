use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    thread,
};

use crate::{
    handle,
    rendering::{
        backend,
        commands::{self, CommandListSubmit},
        frame, image, physical_device,
        render_graph::{self, RenderGraph, RenderGraphExecutor},
    },
    workers,
};

mod vk {
    pub use vulkano::command_buffer::{RenderingAttachmentInfo, RenderingInfo};
}

/// INTERNAL: entry with instance of [render_graph::RenderGraph] and its arguments
struct RenderGraphEntry {
    graph: render_graph::RenderGraph,
    args: BTreeMap<String, render_graph::Arg>,
}

/// Renderer
pub struct Renderer {
    command_list_allocator: commands::CommandListAllocator,

    entries: Mutex<BTreeMap<String, RenderGraphEntry>>,

    backend: Arc<backend::Backend>,
}

impl Renderer {
    /// Creates new instance of [Renderer]
    pub fn new(backend: Arc<backend::Backend>) -> Arc<Renderer> {
        let renderer = Renderer {
            command_list_allocator: commands::CommandListAllocatorFactory::create(backend.as_ref()),
            entries: Default::default(),
            backend,
        };

        Arc::new(renderer)
    }

    /// Adds [render_graph::RenderGraph]
    pub fn add_graph<S, I>(&self, name: S, graph: render_graph::RenderGraph, args: I)
    where
        S: Into<String>,
        I: IntoIterator<Item = (&'static str, render_graph::Arg)>,
    {
        let mut render_graphs = self.entries.lock().unwrap();

        let entry = RenderGraphEntry {
            graph,
            args: args
                .into_iter()
                .map(|(name, arg)| (name.to_string(), arg))
                .collect(),
        };

        render_graphs.insert(name.into(), entry);
    }
}

impl render_graph::RenderGraphExecutor for Renderer {
    fn execute(
        &self,
        frame: &frame::Frame,
        command_list: &mut commands::CommandList,
        graph: &RenderGraph,
        args: &BTreeMap<String, render_graph::Arg>,
    ) {
        let targets: BTreeMap<_, _> = graph
            .targets
            .iter()
            .map(|(target_name, target)| match target {
                render_graph::Target::Swapchain => frame
                    .swapchain
                    .image_views
                    .get(frame.image_index as usize)
                    .map(|image_view| image::ImageView {
                        handle: image_view.clone(),
                        size: frame.swapchain.extent,
                    })
                    .map(|image_view| (target_name.clone(), image_view))
                    .expect("invalid image index"),
            })
            .collect();

        for pass in graph.passes.iter() {
            let rendering_info = vk::RenderingInfo {
                color_attachments: pass
                    .color
                    .iter()
                    .map(|attachment| vk::RenderingAttachmentInfo {
                        load_op: attachment.load_op.into(),
                        clear_value: attachment.load_op.into(),
                        store_op: attachment.store_op.into(),

                        ..vk::RenderingAttachmentInfo::image_view(
                            targets
                                .get(&attachment.target)
                                .map(|target| target.handle.clone())
                                .expect("pass contains color attachment with invalid target"),
                        )
                    })
                    .map(|attachment| Some(attachment))
                    .collect(),

                depth_attachment: pass.depth.as_ref().map(|attachment| {
                    vk::RenderingAttachmentInfo {
                        load_op: attachment.load_op.into(),
                        clear_value: attachment.load_op.into(),
                        store_op: attachment.store_op.into(),

                        ..vk::RenderingAttachmentInfo::image_view(
                            targets
                                .get(&attachment.target)
                                .map(|target| target.handle.clone())
                                .expect("pass contains depth attachment with invalid target"),
                        )
                    }
                }),

                ..Default::default()
            };

            command_list.begin_rendering(rendering_info);

            let context = render_graph::OperationContext {
                command_list,
                args,
                attachments: render_graph::Attachments {
                    color: pass
                        .color
                        .iter()
                        .map(|attachment| {
                            targets
                                .get(&attachment.target)
                                .cloned()
                                .expect("pass contains color attachment with invalid target")
                        })
                        .collect(),

                    depth: pass.depth.as_ref().map(|attachment| {
                        targets
                            .get(&attachment.target)
                            .cloned()
                            .expect("pass contains depth attachment with invalid target")
                    }),
                },
            };

            pass.operation.invoke(context);

            command_list.end_rendering();
        }
    }
}

/// INTERNAL: Renderer worker thread function
fn worker_func(renderer: &Renderer) {
    let entries = renderer.entries.lock().unwrap();

    if entries.is_empty() {
        thread::yield_now();

        return;
    }

    if let Some(frame) = frame::FrameFactory::try_acquire(renderer.backend.as_ref()) {
        let mut command_list = renderer.command_list_allocator.new_list(
            physical_device::QueueFamilyType::Graphics,
            commands::CommandListUsage::Multiple,
        );

        for (_, entry) in entries.iter() {
            renderer.execute(&frame, &mut command_list, &entry.graph, &entry.args);
        }

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
