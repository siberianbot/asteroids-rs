use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::rendering::physical_device::{PhysicalDevice, QueueFamilyType};

mod vk {
    pub use vulkano::device::{
        Device, DeviceCreateInfo, DeviceExtensions, DeviceFeatures, Queue, QueueCreateInfo,
    };
}

/// INTERNAL: list of required device extensions
const REQUIRED_EXTENSIONS: vk::DeviceExtensions = vk::DeviceExtensions {
    khr_swapchain: true,
    khr_dynamic_rendering: true,

    ..vk::DeviceExtensions::empty()
};

/// INTERNAL: list of required device features
const REQUIRED_FEATURES: vk::DeviceFeatures = vk::DeviceFeatures {
    dynamic_rendering: true,

    ..vk::DeviceFeatures::empty()
};

/// Vulkan logical device
#[derive(Clone)]
pub struct LogicalDevice {
    pub handle: Arc<vk::Device>,
    pub queues: BTreeMap<QueueFamilyType, Arc<vk::Queue>>,
}

impl LogicalDevice {
    /// Creates new instance of [LogicalDevice]
    pub fn new(physical_device: &PhysicalDevice) -> LogicalDevice {
        let queue_families = physical_device
            .queue_families
            .values()
            .copied()
            .collect::<BTreeSet<_>>();

        let create_info = vk::DeviceCreateInfo {
            enabled_extensions: REQUIRED_EXTENSIONS,
            enabled_features: REQUIRED_FEATURES,

            queue_create_infos: queue_families
                .iter()
                .map(|index| vk::QueueCreateInfo {
                    queue_family_index: *index,

                    ..Default::default()
                })
                .collect(),

            ..Default::default()
        };

        let (handle, queues) = vk::Device::new(physical_device.handle.clone(), create_info)
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
            queues: physical_device
                .queue_families
                .iter()
                .map(|(queue_family, index)| {
                    (
                        *queue_family,
                        queues.get(index).cloned().expect("invalid queue family"),
                    )
                })
                .collect(),
        };

        logical_device
    }
}
