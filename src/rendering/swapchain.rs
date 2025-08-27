use std::sync::Arc;

use winit::{dpi::PhysicalSize, window::Window};

use crate::rendering::{logical_device::LogicalDevice, physical_device::PhysicalDevice};

mod vk {
    pub use vulkano::{
        image::{Image, ImageUsage, view::ImageView},
        swapchain::{Surface, Swapchain, SwapchainCreateInfo},
    };
}

/// INTERNAL: constructs image views for list of images
fn create_image_views<I>(images: I) -> Vec<Arc<vk::ImageView>>
where
    I: IntoIterator<Item = Arc<vk::Image>>,
{
    images
        .into_iter()
        .map(|image| {
            vk::ImageView::new_default(image)
                .expect("failed to create image view of swapchaing image")
        })
        .collect::<Vec<_>>()
}

/// Vulkan swapchain
#[derive(Clone)]
pub struct Swapchain {
    pub handle: Arc<vk::Swapchain>,
    pub extent: [f32; 2],
    pub image_views: Vec<Arc<vk::ImageView>>,

    window: Arc<Window>,
}

impl Swapchain {
    /// Creates new instance of [Swapchain]
    pub fn new(
        physical_device: &PhysicalDevice,
        logical_device: &LogicalDevice,
        window: Arc<Window>,
        surface: Arc<vk::Surface>,
    ) -> Swapchain {
        let size = window.inner_size().max(PhysicalSize::new(1, 1));

        let create_info = vk::SwapchainCreateInfo {
            image_extent: size.into(),
            image_usage: vk::ImageUsage::COLOR_ATTACHMENT,
            image_format: physical_device.surface_format,
            image_color_space: physical_device.surface_color_space,
            min_image_count: physical_device.surface_image_count,

            ..Default::default()
        };

        let (handle, image_views) =
            vk::Swapchain::new(logical_device.handle.clone(), surface, create_info)
                .map(|(handle, images)| (handle, create_image_views(images)))
                .expect("failed to create swapchain");

        let swapchain = Swapchain {
            handle,
            extent: size.into(),
            image_views,
            window,
        };

        swapchain
    }

    /// Recreates [Swapchain]
    pub fn recreate(self) -> Swapchain {
        let size = self.window.inner_size().max(PhysicalSize::new(1, 1));

        let create_info = vk::SwapchainCreateInfo {
            image_extent: size.into(),

            ..self.handle.create_info()
        };

        let (handle, image_views) = self
            .handle
            .recreate(create_info)
            .map(|(handle, images)| (handle, create_image_views(images)))
            .expect("failed to create swapchain");

        let swapchain = Swapchain {
            handle,
            extent: size.into(),
            image_views,
            window: self.window,
        };

        swapchain
    }
}
