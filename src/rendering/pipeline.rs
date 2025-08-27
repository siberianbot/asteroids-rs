use std::sync::Arc;

mod vk {
    pub use vulkano::{
        Validated, VulkanError,
        device::Device,
        format::Format,
        pipeline::{GraphicsPipeline, graphics::vertex_input::VertexInputRate},
        shader::ShaderModule,
    };
}

/// Type alias of shader factory
pub type ShaderFactory =
    Box<dyn Fn(Arc<vk::Device>) -> Result<Arc<vk::ShaderModule>, vk::Validated<vk::VulkanError>>>;

/// Enumeration of data rates of [InputDataBinding]
#[derive(Clone, Copy)]
pub enum InputDataRate {
    /// Each element of the data source corresponds to a vertex
    PerVertex,
    /// Each element of the data source corresponds to an instance
    PerInstance,
}

impl From<InputDataRate> for vk::VertexInputRate {
    fn from(value: InputDataRate) -> Self {
        match value {
            InputDataRate::PerVertex => Self::Vertex,
            InputDataRate::PerInstance => Self::Instance { divisor: 1 },
        }
    }
}

/// Enumeration of data formats for [InputDataAttribute]
#[derive(Clone, Copy)]
pub enum InputDataFormat {
    Vec2,
}

impl From<InputDataFormat> for vk::Format {
    fn from(value: InputDataFormat) -> Self {
        match value {
            InputDataFormat::Vec2 => Self::R32G32_SFLOAT,
        }
    }
}

/// Input attribute definition in the [InputDataBinding]
pub struct InputDataAttribute {
    /// Attribute offset
    pub offset: usize,
    /// Attribute format
    pub format: InputDataFormat,
}

/// Input definition of [PipelineStage::Vertex] stage
pub struct InputDataBinding {
    /// Stride of input
    pub stride: usize,
    /// Input data rate
    pub rate: InputDataRate,
    /// Attributes of this binding
    pub attributes: Vec<InputDataAttribute>,
}

/// [Pipeline] definition
pub struct PipelineDef {
    /// List of shaders
    pub shaders: Vec<ShaderFactory>,
    /// List of input data bindings
    pub bindings: Vec<InputDataBinding>,
}

/// Graphics pipeline
#[derive(Clone)]
pub struct Pipeline {
    /// VK pipeline handle
    pub handle: Arc<vk::GraphicsPipeline>,
}

/// Trait of a [Pipeline] factory
pub trait PipelineFactory {
    /// Creates instance of [Pipeline] from [PipelineDef]
    fn create(&self, definition: PipelineDef) -> Pipeline;
}
