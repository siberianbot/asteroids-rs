use std::collections::BTreeMap;

use crate::{
    game::entities::EntityId,
    rendering::{
        commands::{self, CommandList},
        frame, image,
    },
};

mod vk {
    pub use vulkano::{
        format::ClearValue,
        render_pass::{AttachmentLoadOp, AttachmentStoreOp},
    };
}

/// List of pass attachments
pub struct Attachments {
    /// Color attachments
    pub color: Vec<image::ImageView>,
    /// Depth attachment
    pub depth: Option<image::ImageView>,
}

/// Pass operation context
pub struct OperationContext<'a> {
    /// Command list to write operations into
    pub command_list: &'a mut commands::CommandList,
    /// List of render graph arguments
    pub args: &'a BTreeMap<String, Arg>,
    /// List of current pass attachments
    pub attachments: Attachments,
}

/// Enumeration of [RenderGraph] execution arguments
#[derive(Clone, Copy)]
pub enum Arg {
    /// Argument is an [EntityId]
    EntityId(EntityId),
}

/// Trait of render graph pass operation
pub trait Operation: Send + Sync {
    /// Invokes operation
    fn invoke(&self, context: OperationContext);
}

/// Stateful implementation of [Operation]
pub struct StatefulOperation<S> {
    state: S,
    delegate: Box<dyn Fn(&S, OperationContext)>,
}

impl<S> StatefulOperation<S> {
    /// Creates new instance of [StatefulOperation]
    pub fn new<F>(state: S, delegate: F) -> StatefulOperation<S>
    where
        F: Fn(&S, OperationContext) + 'static,
    {
        StatefulOperation {
            state,
            delegate: Box::new(delegate),
        }
    }
}

impl<S> Operation for StatefulOperation<S>
where
    S: Send + Sync,
{
    fn invoke(&self, context: OperationContext) {
        (self.delegate)(&self.state, context);
    }
}

unsafe impl<S> Send for StatefulOperation<S> where S: Send + Sync {}

unsafe impl<S> Sync for StatefulOperation<S> where S: Send + Sync {}

/// Enumeration of [RenderGraph] target image
pub enum Target {
    /// Target image is an image from [crate::rendering::swapchain::Swapchain]
    Swapchain,
}

/// Enumeration of attachment clear value
#[derive(Clone, Copy)]
pub enum ClearValue {
    /// Clear value is a RGBA color
    Float([f32; 4]),
}

/// Enumeration of attachment loading operations
#[derive(Clone, Copy)]
pub enum AttachmentLoadOp {
    /// Do nothing with attachment
    Ignore,
    /// Clear target image
    Clear(ClearValue),
    /// Load target image
    Load,
}

impl Into<vk::AttachmentLoadOp> for AttachmentLoadOp {
    fn into(self) -> vk::AttachmentLoadOp {
        match self {
            AttachmentLoadOp::Ignore => vk::AttachmentLoadOp::DontCare,
            AttachmentLoadOp::Clear(_) => vk::AttachmentLoadOp::Clear,
            AttachmentLoadOp::Load => vk::AttachmentLoadOp::Load,
        }
    }
}

impl Into<Option<vk::ClearValue>> for AttachmentLoadOp {
    fn into(self) -> Option<vk::ClearValue> {
        match self {
            AttachmentLoadOp::Clear(clear_value) => match clear_value {
                ClearValue::Float(value) => Some(vk::ClearValue::Float(value)),
            },
            _ => None,
        }
    }
}

/// Enumeration of attachment store operations
#[derive(Clone, Copy)]
pub enum AttachmentStoreOp {
    /// Do nothing with attachment
    Ignore,
    /// Store data in target image
    Store,
}

impl Into<vk::AttachmentStoreOp> for AttachmentStoreOp {
    fn into(self) -> vk::AttachmentStoreOp {
        match self {
            AttachmentStoreOp::Ignore => vk::AttachmentStoreOp::DontCare,
            AttachmentStoreOp::Store => vk::AttachmentStoreOp::Store,
        }
    }
}

/// Attachment of target image
pub struct Attachment {
    /// Name of target
    pub target: String,
    /// Operation on attachment load
    pub load_op: AttachmentLoadOp,
    /// Operation on attachment store
    pub store_op: AttachmentStoreOp,
}

/// Rendering pass
pub struct Pass {
    /// List of color attachments
    pub color: Vec<Attachment>,
    /// Optional depth attachment
    pub depth: Option<Attachment>,
    /// An [Operation] to execute
    pub operation: Box<dyn Operation>,
}

/// [Pass] builder
pub struct PassBuilder {
    color: Vec<Attachment>,
    depth: Option<Attachment>,
    operation: Option<Box<dyn Operation>>,
}

impl PassBuilder {
    /// Adds color attachment to [Pass]
    pub fn add_color(mut self, attachment: Attachment) -> PassBuilder {
        self.color.push(attachment);

        self
    }

    /// Sets depth attachment to [Pass]
    pub fn set_depth(mut self, attachment: Attachment) -> PassBuilder {
        self.depth = Some(attachment);

        self
    }

    /// Sets [Operation] to [Pass]
    pub fn set_operation<O>(mut self, operation: O) -> PassBuilder
    where
        O: Operation + 'static,
    {
        self.operation = Some(Box::new(operation));

        self
    }

    /// INTERNAL: builds [Pass]
    fn build(self) -> Pass {
        Pass {
            color: self.color,
            depth: self.depth,
            operation: self.operation.expect("pass should have operation"),
        }
    }
}

impl Default for PassBuilder {
    fn default() -> Self {
        Self {
            color: Default::default(),
            depth: Default::default(),
            operation: Default::default(),
        }
    }
}

/// Render graph
pub struct RenderGraph {
    /// List of [Target] images
    pub targets: BTreeMap<String, Target>,
    /// List of rendering [Pass]
    pub passes: Vec<Pass>,
}

/// [RenderGraph] builder
pub struct RenderGraphBuilder {
    targets: BTreeMap<String, Target>,
    passes: Vec<Pass>,
}

impl RenderGraphBuilder {
    /// Adds [Target] to [RenderGraph]
    pub fn add_target<N>(mut self, name: N, target: Target) -> RenderGraphBuilder
    where
        N: Into<String>,
    {
        self.targets.insert(name.into(), target);

        self
    }

    /// Adds [Pass] to [RenderGraph]
    pub fn add_pass<B>(mut self, builder: B) -> RenderGraphBuilder
    where
        B: FnOnce(PassBuilder) -> PassBuilder,
    {
        let pass = builder(Default::default()).build();

        self.passes.push(pass);

        self
    }

    /// Builds [RenderGraph]
    pub fn build(self) -> RenderGraph {
        RenderGraph {
            targets: self.targets,
            passes: self.passes,
        }
    }
}

impl Default for RenderGraphBuilder {
    fn default() -> Self {
        Self {
            targets: Default::default(),
            passes: Default::default(),
        }
    }
}

/// Trait of type which able to execute [RenderGraph]
pub trait RenderGraphExecutor {
    /// Executes [RenderGraph]
    fn execute(
        &self,
        frame: &frame::Frame,
        command_list: &mut CommandList,
        graph: &RenderGraph,
        args: &BTreeMap<String, Arg>,
    );
}
