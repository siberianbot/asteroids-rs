use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Add,
    sync::{Arc, Mutex},
};

use vulkano::{
    VulkanLibrary,
    device::{
        Device, DeviceCreateInfo, DeviceExtensions, DeviceFeatures, Queue, QueueCreateInfo,
        QueueFlags,
        physical::{PhysicalDevice as VkPhysicalDevice, PhysicalDeviceType},
    },
    format::Format,
    image::{Image, ImageUsage},
    instance::{Instance, InstanceCreateInfo},
    swapchain::{ColorSpace, Surface, Swapchain as VkSwapchain, SwapchainCreateInfo},
};
use winit::{event_loop::ActiveEventLoop, window::Window};

const DEVICE_EXTENSIONS: DeviceExtensions = DeviceExtensions {
    khr_swapchain: true,
    khr_dynamic_rendering: true,
    ..DeviceExtensions::empty()
};

const DEVICE_FEATURES: DeviceFeatures = DeviceFeatures {
    dynamic_rendering: true,
    ..DeviceFeatures::empty()
};

struct PhysicalDevice {
    handle: Arc<VkPhysicalDevice>,
    device_type: PhysicalDeviceType,
    graphics_queue_family_index: u32,
    present_queue_family_index: u32,
}

impl PhysicalDevice {
    fn try_from(handle: Arc<VkPhysicalDevice>, surface: &Surface) -> Option<PhysicalDevice> {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        enum QueueFamilyType {
            Graphics,
            Present,
        }

        let mut queue_families = BTreeMap::new();

        for (index, properties) in handle.queue_family_properties().iter().enumerate() {
            let index = index as u32;

            if properties.queue_flags.contains(QueueFlags::GRAPHICS) {
                queue_families
                    .entry(QueueFamilyType::Graphics)
                    .or_insert(index);
            }

            if handle
                .surface_support(index, surface)
                .expect("failed to query for surface support")
            {
                queue_families
                    .entry(QueueFamilyType::Present)
                    .or_insert(index);
            }
        }

        if !queue_families.contains_key(&QueueFamilyType::Graphics)
            || !queue_families.contains_key(&QueueFamilyType::Present)
        {
            return None;
        }

        let physical_device = PhysicalDevice {
            device_type: handle.properties().device_type,
            handle,

            graphics_queue_family_index: queue_families
                .get(&QueueFamilyType::Graphics)
                .copied()
                .unwrap(),

            present_queue_family_index: queue_families
                .get(&QueueFamilyType::Present)
                .copied()
                .unwrap(),
        };

        Some(physical_device)
    }
}

struct LogicalDevice {
    handle: Arc<Device>,
    graphics_queue: Arc<Queue>,
    present_queue: Arc<Queue>,
}

impl LogicalDevice {
    fn new(physical_device: &PhysicalDevice) -> LogicalDevice {
        let unique_queue_families = BTreeSet::from([
            physical_device.graphics_queue_family_index,
            physical_device.present_queue_family_index,
        ]);

        let create_info = DeviceCreateInfo {
            enabled_extensions: DEVICE_EXTENSIONS,
            enabled_features: DEVICE_FEATURES,
            queue_create_infos: unique_queue_families
                .iter()
                .copied()
                .map(|index| QueueCreateInfo {
                    queue_family_index: index,
                    ..Default::default()
                })
                .collect(),

            ..Default::default()
        };

        let (handle, queues) = Device::new(physical_device.handle.clone(), create_info)
            .map(|(handle, queues_iter)| {
                (
                    handle,
                    queues_iter
                        .map(|queue| (queue.queue_family_index(), queue))
                        .collect::<BTreeMap<_, _>>(),
                )
            })
            .expect("failed to create logical device");

        let logical_device = LogicalDevice {
            handle,

            graphics_queue: queues
                .get(&physical_device.graphics_queue_family_index)
                .cloned()
                .unwrap(),

            present_queue: queues
                .get(&physical_device.present_queue_family_index)
                .cloned()
                .unwrap(),
        };

        logical_device
    }
}

struct Swapchain {
    window: Arc<Window>,
    handle: Arc<VkSwapchain>,
    images: Vec<Arc<Image>>,
    min_extent: [u32; 2],
    max_extent: [u32; 2],
    format: Format,
}

impl Swapchain {
    fn new(
        physical_device: &PhysicalDevice,
        logical_device: &LogicalDevice,
        surface: Arc<Surface>,
        window: Arc<Window>,
    ) -> Swapchain {
        let size = window.inner_size();

        let surface_capabilities = physical_device
            .handle
            .surface_capabilities(&surface, Default::default())
            .expect("failed to retrieve surface capabilites");
        let surface_formats = physical_device
            .handle
            .surface_formats(&surface, Default::default())
            .expect("failed to retrieve surface formats");
        let (surface_format, surface_color_space) = surface_formats
            .iter()
            .find(|(format, color_space)| {
                *format == Format::R8G8B8A8_SRGB && *color_space == ColorSpace::SrgbNonLinear
            })
            .copied()
            .unwrap_or(surface_formats.first().copied().unwrap());

        let image_count = surface_capabilities
            .max_image_count
            .map(|max_image_count| {
                surface_capabilities
                    .min_image_count
                    .add(1)
                    .clamp(surface_capabilities.min_image_count, max_image_count)
            })
            .unwrap_or_else(|| surface_capabilities.min_image_count.add(1));

        let create_info = SwapchainCreateInfo {
            image_extent: size.into(),
            image_format: surface_format,
            image_color_space: surface_color_space,
            image_usage: ImageUsage::COLOR_ATTACHMENT,
            min_image_count: image_count,

            ..Default::default()
        };

        let (handle, images) =
            VkSwapchain::new(logical_device.handle.clone(), surface, create_info)
                .expect("failed to create swapchain");

        let swapchain = Swapchain {
            window,
            handle,
            images,
            min_extent: surface_capabilities.min_image_extent,
            max_extent: surface_capabilities.max_image_extent,
            format: surface_format,
        };

        swapchain
    }

    fn recreate(&mut self, size: Option<[u32; 2]>) {
        let [width, height] = size.unwrap_or(self.window.inner_size().into());

        let [min_width, min_height] = self.min_extent;
        let [max_width, max_height] = self.max_extent;

        let create_info = SwapchainCreateInfo {
            image_extent: [
                width.clamp(min_width, max_width),
                height.clamp(min_height, max_height),
            ],
            ..self.handle.create_info()
        };

        let (handle, images) = self
            .handle
            .recreate(create_info)
            .expect("failed to recreate swapchain");

        self.handle = handle;
        self.images = images;
    }
}

pub struct Backend {
    physical_device: PhysicalDevice,
    logical_device: LogicalDevice,
    swapchain: Mutex<Swapchain>,
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

        let surface = Surface::from_window(instance.clone(), window.clone())
            .expect("failed to create surface");

        let physical_device = instance
            .enumerate_physical_devices()
            .expect("failed to enumerate physical device")
            .filter_map(|handle| PhysicalDevice::try_from(handle, &surface))
            .min_by_key(|physical_device| match physical_device.device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::VirtualGpu => 1,
                PhysicalDeviceType::IntegratedGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                _ => 99,
            })
            .expect("no suitable physical device available");

        let logical_device = LogicalDevice::new(&physical_device);
        let swapchain = Swapchain::new(&physical_device, &logical_device, surface, window);

        let backend = Backend {
            physical_device,
            logical_device,
            swapchain: Mutex::new(swapchain),
        };

        Arc::new(backend)
    }

    pub fn resize(&self, size: Option<[u32; 2]>) {
        let mut swapchain = self.swapchain.lock().unwrap();

        swapchain.recreate(size);
    }
}
