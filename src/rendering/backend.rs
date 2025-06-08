use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Add,
    sync::{Arc, Mutex, MutexGuard},
};

use vulkano::{
    Validated, VulkanError, VulkanLibrary,
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        PrimaryCommandBufferAbstract,
        allocator::{CommandBufferAllocator, StandardCommandBufferAllocator},
    },
    device::{
        Device, DeviceCreateInfo, DeviceExtensions, DeviceFeatures, Queue, QueueCreateInfo,
        QueueFlags,
        physical::{PhysicalDevice as VkPhysicalDevice, PhysicalDeviceType},
    },
    format::Format,
    image::{Image, ImageUsage, view::ImageView},
    instance::{Instance, InstanceCreateInfo},
    memory::allocator::{
        AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter, StandardMemoryAllocator,
    },
    swapchain::{
        ColorSpace, Surface, Swapchain as VkSwapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
        SwapchainPresentInfo, acquire_next_image,
    },
    sync::{self, GpuFuture},
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
    image_views: Vec<Arc<ImageView>>,
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

        let image_views = images
            .iter()
            .cloned()
            .map(|image| {
                ImageView::new_default(image)
                    .expect("failed to create image view for swapchain image")
            })
            .collect();

        let swapchain = Swapchain {
            window,
            handle,
            images,
            image_views,
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
        self.image_views = self
            .images
            .iter()
            .cloned()
            .map(|image| {
                ImageView::new_default(image)
                    .expect("failed to create image view for swapchain image")
            })
            .collect();
    }
}

pub struct Frame<'a> {
    device: Arc<Device>,
    graphics_queue: Arc<Queue>,
    present_queue: Arc<Queue>,
    swapchain: MutexGuard<'a, Swapchain>,
    image_index: u32,
    image_view: Arc<ImageView>,
    suboptimal: bool,
    acquire_future: SwapchainAcquireFuture,
}

impl Frame<'_> {
    pub fn image_view(&self) -> Arc<ImageView> {
        self.image_view.clone()
    }

    pub fn submit<CB>(mut self, command_buffer: Arc<CB>)
    where
        CB: PrimaryCommandBufferAbstract + 'static,
    {
        let result = sync::now(self.device)
            .join(self.acquire_future)
            .then_execute(self.graphics_queue, command_buffer)
            .expect("failed to execute command buffer")
            .then_swapchain_present(
                self.present_queue,
                SwapchainPresentInfo::swapchain_image_index(
                    self.swapchain.handle.clone(),
                    self.image_index,
                ),
            )
            .then_signal_fence_and_flush()
            .map_err(Validated::unwrap);

        let should_recreate = match result {
            Err(VulkanError::OutOfDate) => true,
            result => {
                result.expect("failed to submit frame").cleanup_finished();
                self.suboptimal
            }
        };

        if !should_recreate {
            return;
        }

        self.swapchain.recreate(None);
    }
}

pub struct Backend {
    physical_device: PhysicalDevice,
    logical_device: LogicalDevice,
    memory_allocator: Arc<dyn MemoryAllocator>,
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
        let memory_allocator = StandardMemoryAllocator::new_default(logical_device.handle.clone());

        let swapchain = Swapchain::new(&physical_device, &logical_device, surface, window);

        let backend = Backend {
            physical_device,
            logical_device,
            memory_allocator: Arc::new(memory_allocator),
            swapchain: Mutex::new(swapchain),
        };

        Arc::new(backend)
    }

    pub fn graphics_queue_family_index(&self) -> u32 {
        self.physical_device.graphics_queue_family_index
    }

    pub fn recreate_swapchain(&self, size: Option<[u32; 2]>) {
        let mut swapchain = self.swapchain.lock().unwrap();

        swapchain.recreate(size);
    }

    pub fn create_command_buffer_allocator(&self) -> Arc<dyn CommandBufferAllocator> {
        let allocator = StandardCommandBufferAllocator::new(
            self.logical_device.handle.clone(),
            Default::default(),
        );

        Arc::new(allocator)
    }

    pub fn create_buffer<Item>(&self, count: usize, usage: BufferUsage) -> Subbuffer<[Item]>
    where
        Item: BufferContents + Sized,
    {
        let create_info = BufferCreateInfo {
            usage,
            ..Default::default()
        };

        let allocation_info = AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::HOST_RANDOM_ACCESS
                | MemoryTypeFilter::PREFER_HOST,
            ..Default::default()
        };

        let buffer = Buffer::new_slice(
            self.memory_allocator.clone(),
            create_info,
            allocation_info,
            count as u64,
        )
        .expect("failed to create buffer");

        buffer
    }

    pub fn acquire_frame(&self) -> Option<Frame> {
        let swapchain = self.swapchain.lock().unwrap();

        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(swapchain.handle.clone(), None).map_err(Validated::unwrap) {
                Err(VulkanError::OutOfDate) => {
                    return None;
                }

                result => result.expect("failed to acquire next frame"),
            };

        let frame = Frame {
            device: self.logical_device.handle.clone(),
            graphics_queue: self.logical_device.graphics_queue.clone(),
            present_queue: self.logical_device.present_queue.clone(),
            image_index,
            image_view: swapchain
                .image_views
                .get(image_index as usize)
                .cloned()
                .expect("invalid swapchain image index"),
            suboptimal,
            acquire_future,
            swapchain,
        };

        Some(frame)
    }
}
