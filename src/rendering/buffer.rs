mod vk {
    pub use vulkano::buffer::{
        BufferContents, BufferReadGuard, BufferUsage, BufferWriteGuard, Subbuffer,
    };
}

/// Enumeration of possible buffer usages
pub enum BufferUsage {
    /// Buffer is used for vertices
    Vertex,
    /// Buffer is used for indices
    Index,
    /// Buffer is used for shader data
    Uniform,
}

impl From<BufferUsage> for vk::BufferUsage {
    fn from(value: BufferUsage) -> Self {
        match value {
            BufferUsage::Vertex => vk::BufferUsage::VERTEX_BUFFER,
            BufferUsage::Index => vk::BufferUsage::INDEX_BUFFER,
            BufferUsage::Uniform => vk::BufferUsage::UNIFORM_BUFFER,
        }
    }
}

/// Enumeration of [Buffer] data source
pub enum BufferData<'a, T>
where
    T: vk::BufferContents + Sized + Clone,
{
    /// Data is a single value
    Value(T),
    /// Data is an empty slice
    EmptySlice(usize),
    /// Data is slice
    Slice(&'a [T]),
}

/// [Buffer] definition
pub struct BufferDef<'a, T>
where
    T: vk::BufferContents + Sized + Clone,
{
    /// Buffer usage
    pub usage: BufferUsage,
    /// Buffer data
    pub data: BufferData<'a, T>,
}

/// Buffer
#[derive(Clone)]
pub struct Buffer<T>
where
    T: vk::BufferContents + Sized,
{
    /// VK buffer handle
    pub handle: vk::Subbuffer<[T]>,
}

impl<T> Buffer<T>
where
    T: vk::BufferContents + Sized,
{
    /// Reads data from buffer
    pub fn read(&self) -> vk::BufferReadGuard<[T]> {
        self.handle.read().unwrap()
    }

    /// Writes data into buffer
    pub fn write(&self) -> vk::BufferWriteGuard<[T]> {
        self.handle.write().unwrap()
    }

    /// Returns length of the buffer
    pub fn len(&self) -> usize {
        self.handle.len() as usize
    }
}

/// Trait of a [Buffer] factory
pub trait BufferFactory {
    /// Creates instance of [Buffer] from [BufferDef]
    fn create<T>(&self, definition: BufferDef<T>) -> Buffer<T>
    where
        T: vk::BufferContents + Sized + Clone;
}
