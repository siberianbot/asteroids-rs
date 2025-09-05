use std::sync::Arc;

use bitflags::bitflags;

mod vk {
    pub use vulkano::{
        format::Format,
        image::{
            Image, ImageUsage,
            view::{ImageView, ImageViewCreateInfo},
        },
    };
}

bitflags! {
    /// Type alias of [Image] usage flags
    pub struct ImageUsage : u8 {
        /// Image can be used as color attachment in [super::render_graph::Pass]
        const COLOR_ATTACHMENT = 1 << 0;
        /// Image can be used as depth attachment in [super::render_graph::Pass]
        const DEPTH_ATTACHMENT = 1 << 1;
    }
}

impl From<ImageUsage> for vk::ImageUsage {
    fn from(value: ImageUsage) -> Self {
        let mut result = vk::ImageUsage::empty();

        result |= vk::ImageUsage::TRANSFER_DST;

        if value.contains(ImageUsage::COLOR_ATTACHMENT) {
            result |= vk::ImageUsage::COLOR_ATTACHMENT;
        }

        if value.contains(ImageUsage::DEPTH_ATTACHMENT) {
            result |= vk::ImageUsage::DEPTH_STENCIL_ATTACHMENT;
        }

        result
    }
}

/// Enumeration of possible [Image] formats
#[derive(Clone, Copy)]
pub enum ImageFormat {
    /// Grayscale
    Y,
    /// Grayscale with alpha channel
    YA,
    /// RGB
    RGB,
    /// RGB with alpha channel
    RGBA,
}

impl From<ImageFormat> for vk::Format {
    fn from(value: ImageFormat) -> Self {
        match value {
            ImageFormat::Y => Self::R8_SNORM,
            ImageFormat::YA => Self::R8G8_SNORM,
            ImageFormat::RGB => Self::R8G8B8_SNORM,
            ImageFormat::RGBA => Self::R8G8B8A8_SNORM,
        }
    }
}

/// [Image] definition
pub struct ImageDef {
    /// Usage flags of image
    pub usage: ImageUsage,
    /// Extent of image
    pub extent: [f32; 2],
    /// Color format of image
    pub format: ImageFormat,
}

/// Image
pub struct Image {
    /// VK image handle
    pub handle: Arc<vk::Image>,
    /// Extent of image
    pub extent: [f32; 2],
}

impl Image {
    /// Creates [ImageView] of this image
    pub fn view(&self) -> ImageView {
        let create_info = vk::ImageViewCreateInfo::from_image(&self.handle);
        let handle = vk::ImageView::new(self.handle.clone(), create_info)
            .expect("failed to create image view");

        let image_view = ImageView {
            handle,
            extent: self.extent,
        };

        image_view
    }
}

/// View of the [Image]
#[derive(Clone)]
pub struct ImageView {
    /// VK image view handle
    pub handle: Arc<vk::ImageView>,
    /// Extent of image view
    pub extent: [f32; 2],
}

/// Trait of [Image] factory
pub trait ImageFactory {
    /// Creates instance of [Image] from [ImageDef]
    fn create(&self, definition: ImageDef) -> Image;
}
