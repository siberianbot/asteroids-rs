use std::sync::MutexGuard;

use vulkano::sync::GpuFuture;

use crate::rendering::{commands, logical_device, physical_device, swapchain};

mod vk {
    pub use vulkano::{
        Validated, VulkanError,
        swapchain::{SwapchainAcquireFuture, SwapchainPresentInfo},
        sync,
    };
}

/// A frame - temporary acquisition of backend resources to render single frame
pub struct Frame<'a> {
    pub logical_device: logical_device::LogicalDevice,
    pub swapchain: MutexGuard<'a, swapchain::Swapchain>,
    pub swapchain_suboptimal: bool,
    pub image_index: u32,
    pub acquire_future: vk::SwapchainAcquireFuture,
}

impl commands::CommandListSubmit for Frame<'_> {
    fn submit(mut self, command_list: commands::CommandList) {
        let present_queue = self
            .logical_device
            .queues
            .get(&physical_device::QueueFamilyType::Present)
            .cloned()
            .expect("queue family is not available");

        let command_buffer = command_list
            .builder
            .build()
            .expect("failed to build command buffer");

        let result = vk::sync::now(self.logical_device.handle.clone())
            .join(self.acquire_future)
            .then_execute(command_list.queue, command_buffer)
            .expect("failed to execute command buffer")
            .then_swapchain_present(
                present_queue,
                vk::SwapchainPresentInfo::swapchain_image_index(
                    self.swapchain.handle.clone(),
                    self.image_index,
                ),
            )
            .then_signal_fence_and_flush()
            .map_err(vk::Validated::unwrap);

        let outdated = match result {
            Err(vk::VulkanError::OutOfDate) => true,
            result => {
                result.expect("failed to present frame").cleanup_finished();
                self.swapchain_suboptimal
            }
        };

        if outdated {
            *self.swapchain = self.swapchain.clone().recreate();
        }
    }
}

/// Trait of [Frame] factory
pub trait FrameFactory {
    /// Tries to acquire [Frame]
    fn try_acquire(&self) -> Option<Frame>;
}
