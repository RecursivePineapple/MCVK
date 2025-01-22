use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use bytemuck::Pod;
use bytemuck::Zeroable;
use vulkano::buffer::AllocateBufferError;
use vulkano::buffer::Buffer;
use vulkano::buffer::BufferUsage;
use vulkano::buffer::Subbuffer;
use vulkano::memory::allocator::MemoryTypeFilter;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::sync::HostAccessError;
use vulkano::Validated;

use crate::vulkan::instance::Allocators;
use crate::vulkan::utils::Ref;

#[derive(Debug, thiserror::Error)]
pub enum UniformError {
    #[error("could not create uniform buffer: {0}")]
    Create(#[from] Validated<AllocateBufferError>),
    #[error("could not upload to uniform buffer: {0}")]
    Upload(HostAccessError),
    #[error("could not fetch from uniform buffer: {0}")]
    Fetch(HostAccessError),
}

pub struct Uniform<T>
where
    T: Send + Sync + Pod + Zeroable,
{
    pub data: T,
    pub uniform: Subbuffer<T>,
}

impl<T> Uniform<T>
where
    T: Send + Sync + Pod + Zeroable,
{
    pub fn new(allocators: &Ref<Allocators>, data: T) -> Result<Self, UniformError> {
        let uniform = Buffer::new_sized::<T>(
            allocators.read().memory_allocator.clone(),
            vulkano::buffer::BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            vulkano::memory::allocator::AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        )?;

        let this = Self { uniform, data };

        this.upload()?;

        Ok(this)
    }

    pub fn upload(&self) -> Result<(), UniformError> {
        let mut guard = self.uniform.write().map_err(UniformError::Upload)?;
        *guard = self.data;
        Ok(())
    }

    pub fn fetch(&mut self) -> Result<(), UniformError> {
        let guard = self.uniform.read().map_err(UniformError::Fetch)?;
        self.data = *guard;
        Ok(())
    }
}

impl<T> Deref for Uniform<T>
where
    T: Send + Sync + Pod + Zeroable,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Uniform<T>
where
    T: Send + Sync + Pod + Zeroable,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
