use std::sync::Arc;

use vulkano::pipeline::Pipeline;

use crate::rendering::pipeline;

mod vk {
    pub use vulkano::descriptor_set::{
        CopyDescriptorSet, DescriptorSet, WriteDescriptorSet, allocator::DescriptorSetAllocator,
    };
}

/// [vk::DescriptorSet] allocator
pub struct DescriptorAllocator {
    /// VK allocator handle
    pub allocator: Arc<dyn vk::DescriptorSetAllocator>,
}

impl DescriptorAllocator {
    /// Allocates [vk::DescriptorSet]
    pub fn allocate<W, C>(
        &self,
        pipeline: &pipeline::Pipeline,
        layout_index: usize,
        writes: W,
        copies: C,
    ) -> Arc<vk::DescriptorSet>
    where
        W: IntoIterator<Item = vk::WriteDescriptorSet>,
        C: IntoIterator<Item = vk::CopyDescriptorSet>,
    {
        let layout = pipeline
            .handle
            .layout()
            .set_layouts()
            .get(layout_index)
            .cloned()
            .expect("invalid layout index");

        let handle = vk::DescriptorSet::new(self.allocator.clone(), layout, writes, copies)
            .expect("failed to create descriptor");

        handle
    }
}

/// Trait of [DescriptorAllocator] factory
pub trait DescriptorAllocatorFactory {
    /// Creates instance of [DescriptorAllocator]
    fn create(&self) -> DescriptorAllocator;
}
