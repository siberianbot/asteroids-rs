use std::sync::Arc;

use vulkano::{
    VulkanLibrary,
    instance::{Instance, InstanceCreateInfo},
    swapchain::Surface,
};
use winit::{event_loop::ActiveEventLoop, window::Window};

pub struct Backend {
    window: Arc<Window>,
    surface: Arc<Surface>,
}

impl Backend {
    pub fn new(event_loop: &ActiveEventLoop, window: Arc<Window>) -> Arc<Backend> {
        let library = VulkanLibrary::new().expect("there is no Vulkan");
        let required_extensions =
            Surface::required_extensions(event_loop).expect("failed to get required extensions");

        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                application_name: Some("asteroids-rs".into()),
                enabled_extensions: required_extensions,

                ..Default::default()
            },
        )
        .expect("failed to create VK instance");

        let surface =
            Surface::from_window(instance, window.clone()).expect("failed to create surface");

        let backend = Backend { window, surface };

        Arc::new(backend)
    }
}
