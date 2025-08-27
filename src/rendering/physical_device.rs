use std::{collections::BTreeMap, ops::Add, sync::Arc};

mod vk {
    pub use vulkano::{
        device::{
            QueueFlags,
            physical::{PhysicalDevice, PhysicalDeviceType},
        },
        format::Format,
        instance::Instance,
        swapchain::{ColorSpace, Surface},
    };
}

/// Enumeration of queue family types
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueueFamilyType {
    /// Graphics queue family
    Graphics,
    /// Present queue family
    Present,
}

/// INTERNAL: list of all required queue family types
const REQUIRED_QUEUE_FAMILY_TYPES: [QueueFamilyType; 2] = [
    QueueFamilyType::Graphics, //
    QueueFamilyType::Present,
];

/// INTERNAL: default color format
const DEFAULT_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;

/// INTERNAL: default color space
const DEFAULT_COLOR_SPACE: vk::ColorSpace = vk::ColorSpace::SrgbNonLinear;

/// Vulkan physical device
pub struct PhysicalDevice {
    pub handle: Arc<vk::PhysicalDevice>,
    pub device_type: vk::PhysicalDeviceType,
    pub queue_families: BTreeMap<QueueFamilyType, u32>,
    pub surface_format: vk::Format,
    pub surface_color_space: vk::ColorSpace,
    pub surface_image_count: u32,
}

impl PhysicalDevice {
    /// Autoselects [PhysicalDevice]
    pub fn autoselect(instance: Arc<vk::Instance>, surface: Arc<vk::Surface>) -> PhysicalDevice {
        let physical_device = instance
            .enumerate_physical_devices()
            .expect("failed to enumerate physical devices")
            .filter_map(|handle| Self::try_from(handle, &surface))
            .min_by_key(|physical_device| match physical_device.device_type {
                vk::PhysicalDeviceType::DiscreteGpu => 0,
                vk::PhysicalDeviceType::VirtualGpu => 1,
                vk::PhysicalDeviceType::IntegratedGpu => 2,
                vk::PhysicalDeviceType::Cpu => 3,
                _ => 99,
            })
            .expect("no suitable physical devices available");

        physical_device
    }

    /// Gets queue family type by index
    pub fn get_queue_family_type(&self, index: u32) -> Option<QueueFamilyType> {
        self.queue_families
            .iter()
            .find(|(_, queue_family_index)| **queue_family_index == index)
            .map(|(queue_family_type, _)| *queue_family_type)
    }

    /// INTERNAL: tries to construct physical device if it supports everything we need
    fn try_from(handle: Arc<vk::PhysicalDevice>, surface: &vk::Surface) -> Option<PhysicalDevice> {
        let queue_families = handle
            .queue_family_properties()
            .iter()
            .enumerate()
            .flat_map(|(index, properties)| {
                let index = index as u32;
                let mut pairs = Vec::new();

                let surface_support = handle
                    .surface_support(index, surface)
                    .expect("failed to query physical device for surface support");

                if properties.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    pairs.push((QueueFamilyType::Graphics, index));
                }

                if surface_support {
                    pairs.push((QueueFamilyType::Present, index));
                }

                pairs
            })
            .fold(
                BTreeMap::new(),
                |mut queue_families, (queue_family_type, index)| {
                    queue_families.entry(queue_family_type).or_insert(index);

                    queue_families
                },
            );

        let has_missing_queue_family = REQUIRED_QUEUE_FAMILY_TYPES
            .iter()
            .any(|queue_family_type| !queue_families.contains_key(queue_family_type));

        if has_missing_queue_family {
            return None;
        }

        let surface_capabilities = handle
            .surface_capabilities(&surface, Default::default())
            .expect("failed to retrieve surface capabilites");

        let surface_formats = handle
            .surface_formats(&surface, Default::default())
            .expect("failed to retrieve surface formats");

        let (surface_format, surface_color_space) = surface_formats
            .iter()
            .find(|format| **format == (DEFAULT_FORMAT, DEFAULT_COLOR_SPACE))
            .copied()
            .unwrap_or_else(|| {
                surface_formats
                    .first()
                    .copied()
                    .expect("no formats are supported by surface")
            });

        let physical_device = PhysicalDevice {
            device_type: handle.properties().device_type,

            handle,
            queue_families,
            surface_format,
            surface_color_space,
            surface_image_count: surface_capabilities
                .max_image_count
                .map(|max_image_count| {
                    surface_capabilities
                        .min_image_count
                        .add(1)
                        .clamp(surface_capabilities.min_image_count, max_image_count)
                })
                .unwrap_or_else(|| surface_capabilities.min_image_count.add(1)),
        };

        Some(physical_device)
    }
}
