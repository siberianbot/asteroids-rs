use std::{
    iter::once,
    sync::{Arc, Mutex},
};

use smallvec::{SmallVec, smallvec};
use winit::{event_loop::ActiveEventLoop, window::Window};

use crate::rendering::{
    buffer, commands, descriptors, frame, logical_device::LogicalDevice,
    physical_device::PhysicalDevice, pipeline, swapchain::Swapchain,
};

mod vk {
    pub use vulkano::{
        Validated, VulkanError, VulkanLibrary,
        buffer::{Buffer, BufferContents, BufferCreateInfo},
        command_buffer::allocator::StandardCommandBufferAllocator,
        descriptor_set::allocator::StandardDescriptorSetAllocator,
        instance::{Instance, InstanceCreateInfo},
        memory::allocator::{
            AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter, StandardMemoryAllocator,
        },
        pipeline::{
            DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
            graphics::{
                GraphicsPipelineCreateInfo,
                color_blend::ColorBlendState,
                input_assembly::{InputAssemblyState, PrimitiveTopology},
                subpass::{PipelineRenderingCreateInfo, PipelineSubpassType},
                vertex_input::{
                    VertexInputAttributeDescription, VertexInputBindingDescription,
                    VertexInputState,
                },
                viewport::ViewportState,
            },
            layout::PipelineDescriptorSetLayoutCreateInfo,
        },
        swapchain::{Surface, acquire_next_image},
    };
}

/// Rendering backend
pub struct Backend {
    physical_device: PhysicalDevice,
    logical_device: LogicalDevice,
    swapchain: Mutex<Swapchain>,

    memory_allocator: Arc<dyn vk::MemoryAllocator>,
}

impl Backend {
    /// Creates new instance of [Backend]
    pub fn new(event_loop: &ActiveEventLoop, window: Arc<Window>) -> Arc<Backend> {
        let required_extensions = vk::Surface::required_extensions(event_loop)
            .expect("failed to retrieve required extensions");

        let library = vk::VulkanLibrary::new().expect("Vulkan library is not available");

        let create_info = vk::InstanceCreateInfo {
            application_name: Some("asteroids-rs".into()),
            enabled_extensions: required_extensions,

            ..Default::default()
        };

        let instance = vk::Instance::new(library, create_info).expect("failed to create instance");
        let surface = vk::Surface::from_window(instance.clone(), window.clone())
            .expect("failed to create surface");

        let physical_device = PhysicalDevice::autoselect(instance, surface.clone());
        let logical_device = LogicalDevice::new(&physical_device);
        let swapchain = Swapchain::new(&physical_device, &logical_device, window, surface);

        let memory_allocator =
            vk::StandardMemoryAllocator::new_default(logical_device.handle.clone());

        let backend = Backend {
            physical_device,
            logical_device,
            swapchain: Mutex::new(swapchain),

            memory_allocator: Arc::new(memory_allocator),
        };

        Arc::new(backend)
    }
}

impl buffer::BufferFactory for Backend {
    fn create<T>(&self, definition: buffer::BufferDef<T>) -> buffer::Buffer<T>
    where
        T: vk::BufferContents + Sized + Clone,
    {
        let create_info = vk::BufferCreateInfo {
            usage: definition.usage.into(),
            ..Default::default()
        };

        let allocation_info = vk::AllocationCreateInfo {
            memory_type_filter: vk::MemoryTypeFilter::HOST_RANDOM_ACCESS,
            ..Default::default()
        };

        let handle = match definition.data {
            buffer::BufferData::Value(value) => vk::Buffer::from_iter(
                self.memory_allocator.clone(),
                create_info,
                allocation_info,
                once(value),
            ),

            buffer::BufferData::EmptySlice(length) => vk::Buffer::new_slice(
                self.memory_allocator.clone(),
                create_info,
                allocation_info,
                length as u64,
            ),

            buffer::BufferData::Slice(items) => vk::Buffer::from_iter(
                self.memory_allocator.clone(),
                create_info,
                allocation_info,
                items.iter().cloned(),
            ),
        };

        let buffer = buffer::Buffer {
            handle: handle.expect("failed to create buffer"),
        };

        buffer
    }
}

impl pipeline::PipelineFactory for Backend {
    fn create(&self, definition: pipeline::PipelineDef) -> pipeline::Pipeline {
        let stages: SmallVec<_> = definition
            .shaders
            .into_iter()
            .map(|shader_factory| {
                let module = shader_factory(self.logical_device.handle.clone())
                    .expect("failed to load shader module");

                let entry_point = module
                    .entry_point("main")
                    .expect("shader has no main entrypoint");

                vk::PipelineShaderStageCreateInfo::new(entry_point)
            })
            .collect();

        let create_info = vk::PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(self.logical_device.handle.clone())
            .expect("failed to construct pipeline layout create info");

        let pipeline_layout =
            vk::PipelineLayout::new(self.logical_device.handle.clone(), create_info)
                .expect("failed to create pipeline layout");

        let vertex_input_state = vk::VertexInputState {
            bindings: definition
                .bindings
                .iter()
                .enumerate()
                .map(|(index, binding)| {
                    let binding = vk::VertexInputBindingDescription {
                        stride: binding.stride as u32,
                        input_rate: binding.rate.into(),

                        ..Default::default()
                    };

                    (index as u32, binding)
                })
                .collect(),

            attributes: definition
                .bindings
                .iter()
                .enumerate()
                .flat_map(|(index, binding)| {
                    binding.attributes.iter().map(move |attribute| {
                        vk::VertexInputAttributeDescription {
                            binding: index as u32,
                            offset: attribute.offset as u32,
                            format: attribute.format.into(),

                            ..Default::default()
                        }
                    })
                })
                .enumerate()
                .map(|(index, attribute)| (index as u32, attribute))
                .collect(),

            ..Default::default()
        };

        let create_info = vk::GraphicsPipelineCreateInfo {
            stages,

            vertex_input_state: Some(vertex_input_state),

            input_assembly_state: Some(vk::InputAssemblyState {
                topology: vk::PrimitiveTopology::TriangleList,
                ..Default::default()
            }),

            viewport_state: Some(vk::ViewportState {
                viewports: smallvec![Default::default()],
                ..Default::default()
            }),

            rasterization_state: Some(Default::default()),
            multisample_state: Some(Default::default()),

            color_blend_state: Some(vk::ColorBlendState::with_attachment_states(
                1,
                Default::default(),
            )),

            dynamic_state: [vk::DynamicState::Viewport].into_iter().collect(),

            subpass: Some(vk::PipelineSubpassType::BeginRendering(
                vk::PipelineRenderingCreateInfo {
                    color_attachment_formats: vec![Some(self.physical_device.surface_format)],
                    ..Default::default()
                },
            )),

            ..vk::GraphicsPipelineCreateInfo::layout(pipeline_layout)
        };

        let handle =
            vk::GraphicsPipeline::new(self.logical_device.handle.clone(), None, create_info)
                .expect("failed to create graphics pipeline");

        let pipeline = pipeline::Pipeline { handle };

        pipeline
    }
}

impl commands::CommandListAllocatorFactory for Backend {
    fn create(&self) -> commands::CommandListAllocator {
        let allocator = vk::StandardCommandBufferAllocator::new(
            self.logical_device.handle.clone(),
            Default::default(),
        );

        let allocator = commands::CommandListAllocator {
            logical_device: self.logical_device.clone(),
            allocator: Arc::new(allocator),
        };

        allocator
    }
}

impl descriptors::DescriptorAllocatorFactory for Backend {
    fn create(&self) -> descriptors::DescriptorAllocator {
        let allocator = vk::StandardDescriptorSetAllocator::new(
            self.logical_device.handle.clone(),
            Default::default(),
        );

        descriptors::DescriptorAllocator {
            allocator: Arc::new(allocator),
        }
    }
}

impl frame::FrameFactory for Backend {
    fn try_acquire(&self) -> Option<frame::Frame> {
        let swapchain = self.swapchain.lock().unwrap();

        let result = vk::acquire_next_image(swapchain.handle.clone(), None);
        let (image_index, suboptimal, acquire_future) = match result.map_err(vk::Validated::unwrap)
        {
            Err(vk::VulkanError::OutOfDate) => {
                return None;
            }

            result => result.expect("failed to acquire next frame"),
        };

        let frame = frame::Frame {
            logical_device: self.logical_device.clone(),
            swapchain,
            swapchain_suboptimal: suboptimal,
            image_index,
            acquire_future,
        };

        Some(frame)
    }
}
