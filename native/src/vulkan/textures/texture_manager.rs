use std::{
    collections::{BTreeSet, HashMap},
    fmt::Debug,
    sync::Arc,
};

use image::{Rgba, RgbaImage};
use num::{FromPrimitive, ToPrimitive};
use num_derive::{FromPrimitive, ToPrimitive};
use smallvec::SmallVec;
use vulkano::{
    buffer::{BufferUsage, Subbuffer},
    command_buffer::{
        AutoCommandBufferBuilder, BlitImageInfo, CopyBufferToImageInfo, PrimaryAutoCommandBuffer,
    },
    device::DeviceOwned,
    image::{Image, ImageFormatInfo, ImageLayout, ImageUsage, SampleCount},
    memory::allocator::{MemoryTypeFilter, StandardMemoryAllocator},
};

use crate::vulkan::{
    instance::Allocators,
    spinlock::SpinLock,
    utils::{Ref, TypedVec},
};

use super::textures::{AnimationMetadata, TextureImage, TextureLoadError};

pub type ArrayIndex = u16;
pub type ArraySlotIndex = u16;

pub struct TextureStorage {
    allocator: Arc<StandardMemoryAllocator>,
    next_array: ArrayIndex,
    arrays: HashMap<ArrayIndex, TextureArray>,
    missingno: Arc<TextureReference>,
}

pub struct TextureArray {
    id: ArrayIndex,
    size: [u32; 2],
    image: Arc<Image>,
    updates: HashMap<ArraySlotIndex, TextureUpdate>,
    free: Arc<SpinLock<BTreeSet<ArraySlotIndex>>>,
    mipmapped: bool,
    mip_levels: u32,
}

#[derive(Debug, Clone)]
pub struct TextureSlotHandle {
    pub array: ArrayIndex,
    pub slot: ArraySlotIndex,
}

struct TextureUpdate {
    image_data: Subbuffer<[u32]>,
    handle: Option<Arc<TextureHandle>>,
}

impl TextureStorage {
    pub fn new(allocators: &Ref<Allocators>) -> Self {
        let mut this = Self {
            allocator: allocators.borrow().memory_allocator.clone(),
            next_array: 0,
            arrays: HashMap::new(),
            missingno: Arc::new(TextureReference::None),
        };

        let missingno = this.allocate(16, 16, 1, true);
        this.enqueue_reference_update(
            &missingno,
            TextureImage::Static {
                image: get_missingno(),
            },
            None,
        )
        .unwrap();
        this.missingno = Arc::new(missingno);

        this
    }

    pub fn get_missingno(&self) -> &Arc<TextureReference> {
        &self.missingno
    }

    pub fn allocate(
        &mut self,
        width: u32,
        height: u32,
        count: u16,
        mipmapped: bool,
    ) -> TextureReference {
        let mut slots = None;

        for (_, array) in &mut self.arrays {
            if array.size[0] == width && array.size[1] == height && array.mipmapped == mipmapped {
                let mut free = array.free.lock();

                if free.len() >= count as usize {
                    slots = Some((
                        array.id,
                        (0..(count as usize))
                            .map(|_| free.pop_first().unwrap())
                            .collect::<SmallVec<[_; 1]>>(),
                        array.free.clone(),
                    ));
                }
            }
        }

        if slots.is_none() {
            let array = self.create_texture_array(width, height, mipmapped, count);

            let mut free = array.free.lock();

            slots = Some((
                array.id,
                (0..(count as usize))
                    .map(|_| free.pop_first().unwrap())
                    .collect::<SmallVec<[_; 1]>>(),
                array.free.clone(),
            ));
        }

        let slots = slots.unwrap();

        TextureReference::Managed(TextureStorageHandle {
            indices: TextureStorageIndices {
                array: slots.0,
                slots: slots.1,
            },
            free: slots.2,
        })
    }

    fn create_texture_array(
        &mut self,
        width: u32,
        height: u32,
        mipmapped: bool,
        min_layers: u16,
    ) -> &mut TextureArray {
        let layers;

        fn get_layers_heuristic(pixels: u32) -> u16 {
            if pixels < 16 * 16 {
                32
            } else if pixels < 64 * 64 {
                16
            } else if pixels < 256 * 256 {
                4
            } else {
                1
            }
        }

        if width == 16 && height == 16 {
            layers = 4096;
        } else if width == 256 && height == 256 {
            layers = 256;
        } else if width == height && width.is_power_of_two() {
            layers = get_layers_heuristic(width * height);
        } else {
            tracing::warn!(what = "a texture array was created with mismatched width and height: it will likely not be re-usable", width, height);
            layers = get_layers_heuristic(width * height);
        }

        let image_properties = self
            .allocator
            .device()
            .physical_device()
            .image_format_properties(ImageFormatInfo {
                usage: ImageUsage::SAMPLED | ImageUsage::TRANSFER_SRC | ImageUsage::TRANSFER_DST,
                format: vulkano::format::Format::A8B8G8R8_UINT_PACK32,
                image_type: vulkano::image::ImageType::Dim2d,
                ..Default::default()
            })
            .unwrap()
            .unwrap();

        let layers = min_layers
            .max(layers)
            .min(image_properties.max_array_layers as u16);

        let mip_levels = if mipmapped {
            if !width.is_power_of_two() || !height.is_power_of_two() {
                tracing::warn!(
                    what =
                        "a texture's width and height must be a power of two to generate mipmaps",
                    width,
                    height
                );
                1
            } else {
                width.max(height).ilog2() + 1
            }
        } else {
            1
        };

        let mip_levels = mip_levels.min(image_properties.max_mip_levels);

        if layers < min_layers {
            panic!("{layers} < {min_layers}");
        }

        let texture = Image::new(
            self.allocator.clone(),
            vulkano::image::ImageCreateInfo {
                extent: [width as u32, height as u32, 1],
                array_layers: layers as u32,
                usage: ImageUsage::SAMPLED | ImageUsage::TRANSFER_SRC | ImageUsage::TRANSFER_DST,
                format: vulkano::format::Format::A8B8G8R8_UINT_PACK32,
                initial_layout: ImageLayout::Undefined,
                image_type: vulkano::image::ImageType::Dim2d,
                samples: SampleCount::Sample1,
                mip_levels,
                ..Default::default()
            },
            vulkano::memory::allocator::AllocationCreateInfo {
                ..Default::default()
            },
        )
        .unwrap(); // TODO: don't unwrap

        let id = self.next_array;
        self.next_array += 1;

        let array = TextureArray {
            id,
            size: [width, height],
            image: texture,
            updates: HashMap::new(),
            free: Arc::new(SpinLock::new((0..layers).collect())),
            mipmapped,
            mip_levels,
        };

        self.arrays.insert(id, array);

        self.arrays.get_mut(&id).unwrap()
    }
}

impl TextureStorage {
    pub fn free(&mut self, indices: TextureStorageIndices) {
        let array = self.arrays.get_mut(&indices.array).unwrap();

        let mut double_freed = SmallVec::<[u16; 1]>::new();

        {
            let mut free = array.free.lock();

            for slot in indices.slots {
                if !free.insert(slot) {
                    double_freed.push(slot);
                }
            }
        }

        for slot in double_freed {
            tracing::warn!(
                what = "texture slot handle was double free'd",
                array = indices.array,
                slot
            );
        }
    }
}

fn get_missingno() -> RgbaImage {
    let black = Rgba(0x00_00_00_FF_u32.to_ne_bytes());
    let pink = Rgba(0xF8_00_F8_FF_u32.to_ne_bytes());

    let mut image = RgbaImage::new(16, 16);

    for y in 0..16 {
        for x in 0..16 {
            let color = (y >= 8) ^ (x >= 8);

            image.put_pixel(x, y, if color { pink } else { black });
        }
    }

    image
}

impl TextureStorage {
    pub fn has_pending_updates(&self) -> bool {
        for array in self.arrays.values() {
            if array.updates.len() > 0 {
                return true;
            }
        }

        false
    }

    pub fn record_commands(
        &mut self,
        buffer: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) {
        let mut update_count = 0;
        let mut invalid_count = 0;

        for (_, array) in &mut self.arrays {
            for (idx, update) in array.updates.drain() {
                let mut copy =
                    CopyBufferToImageInfo::buffer_image(update.image_data, array.image.clone());

                let array_layers = (idx as u32)..((idx + 1) as u32);

                copy.dst_image_layout = ImageLayout::TransferSrcOptimal;
                copy.regions[0].image_subresource.array_layers = array_layers.clone();

                if let Err(e) = buffer.copy_buffer_to_image(copy) {
                    tracing::error!(what = "failed to upload image data to GPU", why = %e, array = array.id, slot = idx);

                    if let Some(handle) = update.handle {
                        handle.texture.set(self.missingno.clone());
                    }

                    invalid_count += 1;

                    continue;
                }

                if array.mipmapped {
                    let mut blit = BlitImageInfo::images(array.image.clone(), array.image.clone());

                    let mut base = blit.regions.remove(0);

                    base.src_subresource.mip_level = 0;
                    base.src_subresource.array_layers = array_layers.clone();
                    base.dst_subresource.array_layers = array_layers.clone();
                    base.src_offsets[1] = [array.size[0], array.size[1], 1];

                    let base = base;

                    for i in 1..array.mip_levels + 1 {
                        let mut region = base.clone();

                        region.dst_subresource.mip_level = i;
                        region.dst_offsets[1] = [array.size[0] >> i, array.size[1] >> i, 1];

                        blit.regions.push(region);
                    }

                    if let Err(e) = buffer.blit_image(blit) {
                        tracing::error!(what = "failed to blit image mipmaps", why = %e, array = array.id, slot = idx);

                        if let Some(handle) = update.handle {
                            handle.texture.set(self.missingno.clone());
                        }

                        invalid_count += 1;

                        continue;
                    }
                }

                update_count += 1;
            }
        }

        tracing::info!(what = "updating gpu textures", update_count, invalid_count);
    }
}

#[derive(Debug, Clone)]
pub struct TextureStorageIndices {
    pub array: ArrayIndex,
    pub slots: SmallVec<[ArraySlotIndex; 1]>,
}

#[derive(Debug)]
/// A reference to one or more GPU textures stored in a TextureStorage.
/// Once dropped, the backing texture slots are freed.
pub struct TextureStorageHandle {
    pub indices: TextureStorageIndices,
    pub free: Arc<SpinLock<BTreeSet<ArraySlotIndex>>>,
}

impl Drop for TextureStorageHandle {
    fn drop(&mut self) {
        self.free.lock().extend(self.indices.slots.iter().copied());
    }
}

#[derive(Debug)]
/// A reference to a GPU texture, or none if no texture is available.
#[allow(dead_code, private_interfaces)]
pub enum TextureReference {
    None,
    Managed(TextureStorageHandle),
    Inexhaustive(Inexhaustive),
}

#[derive(Debug)]
struct Inexhaustive();

#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum TextureFilter {
    Nearest = gl_constants::GL_NEAREST,
    Linear = gl_constants::GL_LINEAR,
    NearestMipmapNearest = gl_constants::GL_NEAREST_MIPMAP_NEAREST,
    LinearMipmapNearest = gl_constants::GL_LINEAR_MIPMAP_NEAREST,
    NearestMipmapLinear = gl_constants::GL_NEAREST_MIPMAP_LINEAR,
    LinearMipmapLinear = gl_constants::GL_LINEAR_MIPMAP_LINEAR,
}

#[derive(Debug, Clone, Copy, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum TextureWrapping {
    ClampToEdge = gl_constants::GL_CLAMP_TO_EDGE,
    ClampToBorder = gl_constants::GL_CLAMP_TO_BORDER,
    MirroredRepeat = gl_constants::GL_MIRRORED_REPEAT,
    Repeat = gl_constants::GL_REPEAT,
    MirrorClampToEdge = gl_constants::GL_MIRROR_CLAMP_TO_EDGE,
}

#[derive(Debug, Clone)]
pub struct TextureParams {
    pub lod_bias: f32,
    pub min_filter: TextureFilter,
    pub mag_filter: TextureFilter,
    pub min_lod: f32,
    pub max_lod: f32,
    pub max_level: u16,
    pub wrap_s: TextureWrapping,
    pub wrap_t: TextureWrapping,
    pub wrap_r: TextureWrapping,
}

impl Default for TextureParams {
    fn default() -> Self {
        Self {
            lod_bias: 0.0,
            min_filter: TextureFilter::NearestMipmapLinear,
            mag_filter: TextureFilter::Linear,
            min_lod: -1000.0,
            max_lod: 1000.0,
            max_level: 1000,
            wrap_s: TextureWrapping::Repeat,
            wrap_t: TextureWrapping::Repeat,
            wrap_r: TextureWrapping::Repeat,
        }
    }
}

#[derive(Debug)]
/// A reference to a minecraft texture. Represents the resource, not the backing texture.
pub struct TextureHandle {
    pub resource_name: Option<String>,
    pub texture_id: u32,
    pub texture: SpinLock<Arc<TextureReference>>,
    pub animation: Option<AnimationMetadata>,
    pub mipmapped: bool,
    pub params: SpinLock<TextureParams>,
}

impl TextureHandle {
    pub fn set_tex_param<N: num::NumCast + Debug>(&self, pname: u32, param: N) {
        let mut l = self.params.lock();

        match pname {
            gl_constants::GL_TEXTURE_LOD_BIAS => match param.to_f32() {
                Some(v) => l.lod_bias = v,
                None => {
                    tracing::warn!(what = "glTexParameter called with invalid param for pname GL_TEXTURE_LOD_BIAS", param = ?param);
                }
            },
            gl_constants::GL_TEXTURE_MIN_FILTER => {
                match param.to_u32().and_then(TextureFilter::from_u32) {
                    Some(v) => {
                        l.min_filter = v;
                    }
                    None => {
                        tracing::warn!(what = "glTexParameter called with invalid param for pname GL_TEXTURE_MIN_FILTER", param = ?param);
                    }
                }
            }
            gl_constants::GL_TEXTURE_MAG_FILTER => {
                match param.to_u32().and_then(TextureFilter::from_u32) {
                    Some(v) => {
                        l.mag_filter = v;
                    }
                    None => {
                        tracing::warn!(what = "glTexParameter called with invalid param for pname GL_TEXTURE_MAG_FILTER", param = ?param);
                    }
                }
            }
            gl_constants::GL_TEXTURE_MIN_LOD => match param.to_f32() {
                Some(v) => l.min_lod = v,
                None => {
                    tracing::warn!(what = "glTexParameter called with invalid param for pname GL_TEXTURE_MIN_LOD", param = ?param);
                }
            },
            gl_constants::GL_TEXTURE_MAX_LOD => match param.to_f32() {
                Some(v) => l.max_lod = v,
                None => {
                    tracing::warn!(what = "glTexParameter called with invalid param for pname GL_TEXTURE_MAX_LOD", param = ?param);
                }
            },
            gl_constants::GL_TEXTURE_WRAP_S => {
                match param.to_u32().and_then(TextureWrapping::from_u32) {
                    Some(v) => {
                        l.wrap_s = v;
                    }
                    None => {
                        tracing::warn!(what = "glTexParameter called with invalid param for pname GL_TEXTURE_WRAP_S", param = ?param);
                    }
                }
            }
            gl_constants::GL_TEXTURE_WRAP_T => {
                match param.to_u32().and_then(TextureWrapping::from_u32) {
                    Some(v) => {
                        l.wrap_t = v;
                    }
                    None => {
                        tracing::warn!(what = "glTexParameter called with invalid param for pname GL_TEXTURE_WRAP_T", param = ?param);
                    }
                }
            }
            gl_constants::GL_TEXTURE_WRAP_R => {
                match param.to_u32().and_then(TextureWrapping::from_u32) {
                    Some(v) => {
                        l.wrap_r = v;
                    }
                    None => {
                        tracing::warn!(what = "glTexParameter called with invalid param for pname GL_TEXTURE_WRAP_R", param = ?param);
                    }
                }
            }
            _ => {
                tracing::warn!(what = "glTexParameter() called with unsupported pname", pname = pname, param = ?param);
            }
        }
    }

    pub fn get_tex_param<N: num::Num + num::NumCast + Debug>(&self, pname: u32) -> N {
        let l = self.params.lock();

        match pname {
            gl_constants::GL_TEXTURE_LOD_BIAS => N::from(l.lod_bias).unwrap_or(N::zero()),
            gl_constants::GL_TEXTURE_MIN_FILTER => N::from(l.min_filter).unwrap_or(N::zero()),
            gl_constants::GL_TEXTURE_MAG_FILTER => N::from(l.mag_filter).unwrap_or(N::zero()),
            gl_constants::GL_TEXTURE_MIN_LOD => N::from(l.min_lod).unwrap_or(N::zero()),
            gl_constants::GL_TEXTURE_MAX_LOD => N::from(l.max_lod).unwrap_or(N::zero()),
            gl_constants::GL_TEXTURE_WRAP_S => N::from(l.wrap_s).unwrap_or(N::zero()),
            gl_constants::GL_TEXTURE_WRAP_T => N::from(l.wrap_t).unwrap_or(N::zero()),
            gl_constants::GL_TEXTURE_WRAP_R => N::from(l.wrap_r).unwrap_or(N::zero()),
            _ => {
                tracing::warn!(
                    what = "glGetTexParameter() called with unsupported pname",
                    pname = pname
                );
                N::zero()
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TextureError {
    #[error("handle was invalid: handle did not contain a texture")]
    NoTexture,
    #[error("handle was invalid: handle must contain a managed texture")]
    BadHandle,
    #[error("handle or image was invalid: image has {0} frames while the handle has {1} frames")]
    LengthMismatch(usize, usize),
    #[error("could not load texture: {0}")]
    LoadError(#[from] TextureLoadError),
}

impl TextureStorage {
    pub fn enqueue_indices_update(
        &mut self,
        indices: &TextureStorageIndices,
        image: TextureImage,
        owning_handle: Option<Arc<TextureHandle>>,
    ) -> Result<(), TextureError> {
        let image = image.load()?;

        image.validate()?;

        let frames = image.get_frames();

        if indices.slots.len() != frames.len() {
            return Err(TextureError::LengthMismatch(
                frames.len(),
                indices.slots.len(),
            ));
        }

        let frame_pixel_size = (image.width() * image.height()) as usize;

        let mut image_data = Vec::with_capacity(frame_pixel_size * frames.len());

        for frame in &frames {
            for pixel in frame.pixels() {
                let [r, g, b, a] = pixel.0.clone();
                image_data.push(u32::from_ne_bytes([a, b, g, r]));
            }
        }

        let source_buffer = vulkano::buffer::Buffer::new_slice::<u32>(
            self.allocator.clone(),
            vulkano::buffer::BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            vulkano::memory::allocator::AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            image_data.len() as u64,
        )
        .unwrap(); // TODO: don't unwrap

        {
            let mut guard = source_buffer.write().unwrap();
            guard.copy_from_slice(&image_data);
        }

        let array = self.arrays.get_mut(&indices.array).unwrap();

        for (i, slot) in indices.slots.iter().enumerate() {
            array.updates.insert(
                *slot,
                TextureUpdate {
                    image_data: source_buffer
                        .clone()
                        .slice((i * frame_pixel_size) as u64..((i + 1) * frame_pixel_size) as u64),
                    handle: owning_handle.clone(),
                },
            );
        }

        Ok(())
    }

    pub fn enqueue_reference_update(
        &mut self,
        tex_ref: &TextureReference,
        image: TextureImage,
        owning_handle: Option<Arc<TextureHandle>>,
    ) -> Result<(), TextureError> {
        let texture = match tex_ref {
            TextureReference::None => None,
            TextureReference::Managed(tex) => {
                let array = self.arrays.get(&tex.indices.array).unwrap();

                if array.size[0] != image.width()
                    || array.size[1] != image.height()
                    || tex.indices.slots.len() != image.get_frames().len()
                {
                    // image isn't compatible with the allocated texture, allocate another one
                    None
                } else {
                    Some(tex)
                }
            }
            _ => {
                return Err(TextureError::BadHandle);
            }
        };

        #[allow(unused_assignments)]
        let mut temp = None;

        let texture = match texture {
            Some(tex) => tex,
            None => {
                if let Some(handle) = owning_handle.as_ref() {
                    let tex_ref = Arc::new(self.allocate(
                        image.width(),
                        image.height(),
                        image.get_frames().len() as u16,
                        handle.mipmapped,
                    ));

                    handle.texture.set(tex_ref.clone());

                    temp = Some(tex_ref);

                    match temp.as_ref().unwrap().as_ref() {
                        TextureReference::Managed(tex) => tex,
                        _ => panic!(),
                    }
                } else {
                    return Err(TextureError::NoTexture);
                }
            }
        };

        self.enqueue_indices_update(&texture.indices, image, owning_handle)
    }

    pub fn enqueue_handle_update(
        &mut self,
        tex_handle: &Arc<TextureHandle>,
        image: TextureImage,
    ) -> Result<(), TextureError> {
        let texture = tex_handle.texture.get();

        self.enqueue_reference_update(&texture, image, Some(tex_handle.clone()))
    }
}
