use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Add,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
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
    pipeline::{
        DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
        graphics::{
            GraphicsPipelineCreateInfo,
            color_blend::ColorBlendState,
            subpass::{PipelineRenderingCreateInfo, PipelineSubpassType},
            vertex_input::{Vertex as VertexTrait, VertexDefinition},
            viewport::{Viewport, ViewportState},
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
    },
    shader::{EntryPoint, ShaderModule},
    swapchain::{
        ColorSpace, Surface, Swapchain as VkSwapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
        SwapchainPresentInfo, acquire_next_image,
    },
    sync::{self, GpuFuture},
};
use winit::{event_loop::ActiveEventLoop, window::Window};

use crate::{
    dispatch::{Dispatcher, Event},
    rendering::shaders::Vertex,
};

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

struct SwapchainInner {
    handle: Arc<VkSwapchain>,
    images: Vec<Arc<Image>>,
    image_views: Vec<Arc<ImageView>>,
    extent: [u32; 2],
}

struct Swapchain {
    window: Arc<Window>,
    min_extent: [u32; 2],
    max_extent: [u32; 2],
    format: Format,
    outdated: Arc<AtomicBool>,
    inner: Mutex<SwapchainInner>,
}

impl Swapchain {
    fn new(
        physical_device: &PhysicalDevice,
        logical_device: &LogicalDevice,
        surface: Arc<Surface>,
        window: Arc<Window>,
    ) -> Arc<Swapchain> {
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

        let inner = SwapchainInner {
            handle,
            images,
            image_views,
            extent: size.into(),
        };

        let swapchain = Swapchain {
            window,
            min_extent: surface_capabilities.min_image_extent,
            max_extent: surface_capabilities.max_image_extent,
            format: surface_format,
            outdated: Arc::new(AtomicBool::new(false)),
            inner: Mutex::new(inner),
        };

        Arc::new(swapchain)
    }

    fn recreate(&self) {
        let [width, height]: [u32; 2] = self.window.inner_size().into();

        let [min_width, min_height] = self.min_extent;
        let [max_width, max_height] = self.max_extent;

        let mut inner = self.inner.lock().unwrap();

        let create_info = SwapchainCreateInfo {
            image_extent: [
                width.clamp(min_width, max_width),
                height.clamp(min_height, max_height),
            ],
            ..inner.handle.create_info()
        };

        let (handle, images) = inner
            .handle
            .recreate(create_info)
            .expect("failed to recreate swapchain");

        *inner = SwapchainInner {
            handle,
            image_views: images
                .iter()
                .cloned()
                .map(|image| {
                    ImageView::new_default(image)
                        .expect("failed to create image view for swapchain image")
                })
                .collect(),
            images,
            extent: [width, height],
        };

        self.outdated.store(false, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShaderStage {
    Vertex,
    Fragment,
}

pub type ShaderFactory = fn(Arc<Device>) -> Result<Arc<ShaderModule>, Validated<VulkanError>>;

pub struct Frame<'a> {
    device: Arc<Device>,
    graphics_queue: Arc<Queue>,
    present_queue: Arc<Queue>,
    swapchain_outdated: Arc<AtomicBool>,
    swapchain_inner: MutexGuard<'a, SwapchainInner>,
    extent: [f32; 2],
    image_index: u32,
    image_view: Arc<ImageView>,
    suboptimal: bool,
    acquire_future: SwapchainAcquireFuture,
}

impl Frame<'_> {
    pub fn extent(&self) -> [f32; 2] {
        self.extent
    }

    pub fn image_view(&self) -> Arc<ImageView> {
        self.image_view.clone()
    }

    pub fn submit<CB>(self, command_buffer: Arc<CB>)
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
                    self.swapchain_inner.handle.clone(),
                    self.image_index,
                ),
            )
            .then_signal_fence_and_flush()
            .map_err(Validated::unwrap);

        let outdated = match result {
            Err(VulkanError::OutOfDate) => true,
            result => {
                result.expect("failed to submit frame").cleanup_finished();
                self.suboptimal
            }
        };

        self.swapchain_outdated.store(outdated, Ordering::Relaxed);
    }
}

pub struct Backend {
    physical_device: PhysicalDevice,
    logical_device: LogicalDevice,
    memory_allocator: Arc<dyn MemoryAllocator>,
    swapchain: Arc<Swapchain>,
}

impl Backend {
    pub fn new(
        event_dispatcher: &Dispatcher<Event>,
        event_loop: &ActiveEventLoop,
        window: Arc<Window>,
    ) -> Arc<Backend> {
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

        event_dispatcher.add_handler(move |event| if let Event::WindowResized(size) = event {});

        let backend = Backend {
            physical_device,
            logical_device,
            memory_allocator: Arc::new(memory_allocator),
            swapchain,
        };

        Arc::new(backend)
    }

    pub fn graphics_queue_family_index(&self) -> u32 {
        self.physical_device.graphics_queue_family_index
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

    pub fn create_pipeline<I, F>(&self, shaders_iter: I) -> Arc<GraphicsPipeline>
    where
        I: IntoIterator<Item = (ShaderStage, F)>,
        F: FnOnce(Arc<Device>) -> Result<Arc<ShaderModule>, Validated<VulkanError>>,
    {
        let shaders: BTreeMap<ShaderStage, (Arc<ShaderModule>, EntryPoint)> = shaders_iter
            .into_iter()
            .map(|(stage, shader_factory)| {
                let module = shader_factory(self.logical_device.handle.clone())
                    .expect("failed to construct shader");
                let entry_point = module
                    .entry_point("main")
                    .expect("shader has no main function");

                (stage, (module, entry_point))
            })
            .collect();

        let (_, vertex_entry_point) = shaders
            .get(&ShaderStage::Vertex)
            .expect("there is no vertex shader stage");
        let vertex_input_state = Vertex::per_vertex()
            .definition(vertex_entry_point)
            .expect("failed to construct vertex input state");

        let stages: Vec<_> = shaders
            .iter()
            .map(|(_, (_, entry_point))| PipelineShaderStageCreateInfo::new(entry_point.clone()))
            .collect();

        let create_info = PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(self.logical_device.handle.clone())
            .expect("failed to construct pipeline create info");
        let pipeline_layout = PipelineLayout::new(self.logical_device.handle.clone(), create_info)
            .expect("failed to create pipeline layout");

        let create_info = GraphicsPipelineCreateInfo {
            stages: stages.into_iter().collect(),
            vertex_input_state: Some(vertex_input_state),
            input_assembly_state: Some(Default::default()),
            rasterization_state: Some(Default::default()),
            multisample_state: Some(Default::default()),
            viewport_state: Some(ViewportState {
                viewports: vec![Viewport::default()].into(),

                ..Default::default()
            }),
            color_blend_state: Some(ColorBlendState::with_attachment_states(
                1,
                Default::default(),
            )),
            dynamic_state: [DynamicState::Viewport].into_iter().collect(),
            subpass: Some(PipelineSubpassType::BeginRendering(
                PipelineRenderingCreateInfo {
                    color_attachment_formats: vec![Some(self.swapchain.format)],
                    ..PipelineRenderingCreateInfo::default()
                },
            )),

            ..GraphicsPipelineCreateInfo::layout(pipeline_layout)
        };
        let pipeline = GraphicsPipeline::new(self.logical_device.handle.clone(), None, create_info)
            .expect("failed to create graphics pipeline");

        pipeline
    }

    pub fn acquire_frame(&self) -> Option<Frame> {
        if self.swapchain.outdated.load(Ordering::Relaxed) {
            self.swapchain.recreate();
        }

        let swapchain_inner = self.swapchain.inner.lock().unwrap();

        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(swapchain_inner.handle.clone(), None)
                .map_err(Validated::unwrap)
            {
                Err(VulkanError::OutOfDate) => {
                    return None;
                }

                result => result.expect("failed to acquire next frame"),
            };

        let [width, height] = swapchain_inner.extent;

        let frame = Frame {
            device: self.logical_device.handle.clone(),
            graphics_queue: self.logical_device.graphics_queue.clone(),
            present_queue: self.logical_device.present_queue.clone(),
            extent: [width as f32, height as f32],
            image_index,
            image_view: swapchain_inner
                .image_views
                .get(image_index as usize)
                .cloned()
                .expect("invalid swapchain image index"),
            suboptimal,
            acquire_future,
            swapchain_outdated: self.swapchain.outdated.clone(),
            swapchain_inner,
        };

        Some(frame)
    }
}
