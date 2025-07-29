use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use vulkano::{
    Validated, VulkanError,
    buffer::{BufferUsage, Subbuffer},
    device::Device,
    pipeline::{GraphicsPipeline, graphics::input_assembly::PrimitiveTopology},
    shader::ShaderModule,
};

use crate::{
    assets::types::Vertex,
    rendering::backend::{Backend, ShaderStage},
};

pub mod types;

/// Mesh asset data
pub struct MeshAsset {
    /// Vertex buffer
    pub vertex: Subbuffer<[Vertex]>,
    /// Index buffer
    pub index: Subbuffer<[u32]>,
}

/// Pipeline asset data
pub struct PipelineAsset {
    /// Pipeline
    pub pipeline: Arc<GraphicsPipeline>,
}

/// An asset
///
/// See content of next structures for specific details:
/// * [MeshAsset]
/// * [PipelineAsset]
pub enum Asset {
    /// Variant of an asset with [MeshAsset] data
    Mesh(MeshAsset),
    /// Variant of an asset with [PipelineAsset] data
    Pipeline(PipelineAsset),
}

impl Asset {
    /// Returns reference to [MeshAsset] if [Asset] is a [Asset::Mesh] variant
    pub fn as_mesh(&self) -> Option<&MeshAsset> {
        if let Asset::Mesh(mesh) = self {
            Some(mesh)
        } else {
            None
        }
    }

    /// Returns reference to [PipelineAsset] if [Asset] is a [Asset::Pipeline] variant
    pub fn as_pipeline(&self) -> Option<&PipelineAsset> {
        if let Asset::Pipeline(pipeline) = self {
            Some(pipeline)
        } else {
            None
        }
    }
}

impl From<MeshAsset> for Asset {
    fn from(value: MeshAsset) -> Self {
        Self::Mesh(value)
    }
}

impl From<PipelineAsset> for Asset {
    fn from(value: PipelineAsset) -> Self {
        Self::Pipeline(value)
    }
}

/// Context for [IntoAsset::into_asset] trait method
pub struct IntoAssetContext {
    backend: Arc<Backend>,
}

/// Trait of type, which can construct instance of [Asset] from its definition
pub trait IntoAsset {
    /// Converts definition into [Asset] instance
    fn into_asset(self, context: IntoAssetContext) -> Asset;
}

/// Definition of [MeshAsset]
pub struct MeshAssetDef {
    /// List of vertices
    pub vertices: Vec<Vertex>,
    /// List of indices
    pub indices: Vec<u32>,
}

impl IntoAsset for MeshAssetDef {
    fn into_asset(self, context: IntoAssetContext) -> Asset {
        let mesh = MeshAsset {
            vertex: context
                .backend
                .create_buffer_iter(BufferUsage::VERTEX_BUFFER, self.vertices.into_iter()),

            index: context
                .backend
                .create_buffer_iter(BufferUsage::INDEX_BUFFER, self.indices.into_iter()),
        };

        mesh.into()
    }
}

/// Trait alias for shader factory function
pub trait ShaderFactory:
    FnOnce(Arc<Device>) -> Result<Arc<ShaderModule>, Validated<VulkanError>>
{
}

/// Definition of [PipelineAsset]
pub struct PipelineAssetDef {
    /// Primitive topology
    pub topology: PrimitiveTopology,
    /// List of shaders
    pub shaders: Vec<(ShaderStage, Box<dyn ShaderFactory>)>,
}

impl IntoAsset for PipelineAssetDef {
    fn into_asset(self, context: IntoAssetContext) -> Asset {
        let pipeline = PipelineAsset {
            pipeline: context
                .backend
                .create_pipeline(self.topology, self.shaders.into_iter()),
        };

        pipeline.into()
    }
}

/// Reference to asset
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssetRef(String);

impl From<&str> for AssetRef {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for AssetRef {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Assets infrastructure
pub struct Assets {
    backend: Arc<Backend>,
    assets: RwLock<BTreeMap<AssetRef, Arc<Asset>>>,
}

impl Assets {
    /// Creates new instance of [Assets]
    pub fn new(backend: Arc<Backend>) -> Arc<Assets> {
        let assets = Assets {
            backend,
            assets: Default::default(),
        };

        Arc::new(assets)
    }

    /// Gets asset by its reference
    pub fn get(&self, asset_ref: &AssetRef) -> Option<Arc<Asset>> {
        let assets = self.assets.read().unwrap();

        assets.get(asset_ref).cloned()
    }

    /// Loads asset
    pub fn load<A>(&self, asset_ref: AssetRef, asset: A)
    where
        A: IntoAsset,
    {
        let mut assets = self.assets.write().unwrap();

        let context = IntoAssetContext {
            backend: self.backend.clone(),
        };

        let asset = asset.into_asset(context);

        assets.insert(asset_ref, Arc::new(asset));
    }

    /// Unloads asset
    pub fn unload(&self, asset_ref: &AssetRef) {
        let mut assets = self.assets.write().unwrap();

        assets.remove(asset_ref);
    }
}
