use std::sync::Arc;

use vulkano::pipeline::Pipeline;

use crate::rendering::{buffer, logical_device, physical_device, pipeline};

mod vk {
    pub use vulkano::{
        buffer::BufferContents,
        command_buffer::{
            AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderingInfo,
            allocator::CommandBufferAllocator,
        },
        descriptor_set::DescriptorSetWithOffsets,
        device::Queue,
        pipeline::graphics::viewport::Viewport,
    };
}

/// Enumeration of usages of [CommandList]
pub enum CommandListUsage {
    Once,
    Multiple,
}

impl From<CommandListUsage> for vk::CommandBufferUsage {
    fn from(value: CommandListUsage) -> Self {
        match value {
            CommandListUsage::Once => Self::OneTimeSubmit,
            CommandListUsage::Multiple => Self::MultipleSubmit,
        }
    }
}

/// [CommandList] allocator
pub struct CommandListAllocator {
    /// [logical_device::LogicalDevice] instance
    pub logical_device: logical_device::LogicalDevice,
    /// VK command buffer allocator instance
    pub allocator: Arc<dyn vk::CommandBufferAllocator>,
}

impl CommandListAllocator {
    /// Creates new instance of [CommandList] to be executed in specified queue
    pub fn new_list(
        &self,
        queue_family: physical_device::QueueFamilyType,
        usage: CommandListUsage,
    ) -> CommandList {
        let queue = self
            .logical_device
            .queues
            .get(&queue_family)
            .cloned()
            .expect("queue family is not available");

        let builder = vk::AutoCommandBufferBuilder::primary(
            self.allocator.clone(),
            queue.queue_family_index(),
            usage.into(),
        )
        .expect("failed to create command buffer builder");

        CommandList { builder, queue }
    }
}

/// Trait of a [CommandListAllocator] factory
pub trait CommandListAllocatorFactory {
    /// Creates new instance of [CommandListAllocator]
    fn create(&self) -> CommandListAllocator;
}

/// List of commands to be executed by GPU
pub struct CommandList {
    /// Vulkano command buffer builder
    pub builder: vk::AutoCommandBufferBuilder<vk::PrimaryAutoCommandBuffer>,
    /// Target VK queue
    pub queue: Arc<vk::Queue>,
}

impl CommandList {
    /// Adds command to begin rendering
    pub fn begin_rendering(&mut self, rendering_info: vk::RenderingInfo) {
        self.builder
            .begin_rendering(rendering_info)
            .expect("failed to add begin rendering command");
    }

    /// Adds command to finish rendering
    pub fn end_rendering(&mut self) {
        self.builder
            .end_rendering()
            .expect("failed to add end rendering command");
    }

    /// Adds command to set viewports
    pub fn set_viewports<I>(&mut self, viewports: I)
    where
        I: IntoIterator<Item = vk::Viewport>,
    {
        let viewports = viewports.into_iter().collect();

        self.builder
            .set_viewport(0, viewports)
            .expect("failed to add set viewport command");
    }

    /// Adds command to bind pipeline
    pub fn bind_pipeline(&mut self, pipeline: &pipeline::Pipeline) {
        self.builder
            .bind_pipeline_graphics(pipeline.handle.clone())
            .expect("failed to add bind graphics pipeline command");
    }

    /// Adds command to bind vertex buffer
    pub fn bind_vertex_buffer<T>(&mut self, buffer: &buffer::Buffer<T>)
    where
        T: vk::BufferContents + Sized,
    {
        self.builder
            .bind_vertex_buffers(0, buffer.handle.clone())
            .expect("failed to add bind vertex buffer command");
    }

    /// Adds command to bind index buffer
    pub fn bind_index_buffer(&mut self, buffer: &buffer::Buffer<u32>) {
        self.builder
            .bind_index_buffer(buffer.handle.clone())
            .expect("failed to add bind index buffer command");
    }

    /// Adds command to bind descriptors
    pub fn bind_descriptors<I, T>(&mut self, pipeline: &pipeline::Pipeline, descriptors: I)
    where
        I: IntoIterator<Item = T>,
        T: Into<vk::DescriptorSetWithOffsets>,
    {
        let descriptors = descriptors.into_iter().collect::<Vec<_>>();

        self.builder
            .bind_descriptor_sets(
                pipeline.handle.bind_point(),
                pipeline.handle.layout().clone(),
                0,
                descriptors,
            )
            .expect("failed to add bind descriptors command");
    }

    /// Adds command to draw
    pub fn draw(&mut self, index_count: usize, instance_count: usize) {
        unsafe {
            self.builder
                .draw_indexed(index_count as u32, instance_count as u32, 0, 0, 0)
                .expect("failed to add draw indexed command");
        }
    }
}

/// Trait of object able to submit [CommandList]
pub trait CommandListSubmit {
    /// Submits [CommandList]
    fn submit(self, command_list: CommandList);
}
