use std::sync::Arc;

mod vk {
    pub use vulkano::image::view::ImageView;
}

/// View of the image
#[derive(Clone)]
pub struct ImageView {
    /// VK image view handle
    pub handle: Arc<vk::ImageView>,

    /// Size of image view
    pub size: [f32; 2],
}
