use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use derivative::Derivative;
use image::Rgba;
use image::RgbaImage;
use num::FromPrimitive;
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use smallvec::SmallVec;
use tracing::info;
use tracing::warn;
use vulkano::buffer::BufferUsage;
use vulkano::buffer::Subbuffer;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::BlitImageInfo;
use vulkano::command_buffer::CommandBufferUsage;
use vulkano::command_buffer::CopyBufferToImageInfo;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use vulkano::command_buffer::PrimaryCommandBufferAbstract;
use vulkano::device::DeviceOwned;
use vulkano::image::sampler::Sampler;
use vulkano::image::view::ImageView;
use vulkano::image::view::ImageViewCreateInfo;
use vulkano::image::view::ImageViewType;
use vulkano::image::Image;
use vulkano::image::ImageAspects;
use vulkano::image::ImageFormatInfo;
use vulkano::image::ImageLayout;
use vulkano::image::ImageSubresourceRange;
use vulkano::image::ImageUsage;
use vulkano::image::SampleCount;
use vulkano::memory::allocator::MemoryTypeFilter;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::sync::GpuFuture;

use crate::vulkan::instance::Allocators;
use crate::vulkan::render_manager::RenderManager;
use crate::vulkan::spinlock::SpinLock;
use crate::vulkan::utils::Ref;

use super::lookup::TextureLookup;
use super::textures::AnimationMetadata;
use super::textures::TextureImage;
use super::textures::TextureLoadError;

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
    layer_count: u16,
    size: [u32; 2],
    image: Arc<Image>,
    updates: HashMap<ArraySlotIndex, TextureUpdate>,
    free: Arc<SpinLock<BTreeSet<ArraySlotIndex>>>,
    mipmapped: bool,
    mip_levels: u32,
}

struct TextureUpdate {
    image_data: Subbuffer<[u32]>,
    handle: Option<Arc<TextureHandle>>,
    animation: Option<AnimationMetadata>,
}

impl TextureStorage {
    pub fn new(allocators: &Ref<Allocators>) -> Self {
        let mut this = Self {
            allocator: allocators.read().memory_allocator.clone(),
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
        .unwrap();

        let id = self.next_array;
        self.next_array += 1;

        let array = TextureArray {
            id,
            layer_count: layers,
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
    pub fn get_view(&self, array: ArrayIndex) -> Arc<ImageView> {
        let array = self.arrays.get(&array).unwrap();
        let image = array.image.clone();

        ImageView::new(
            image.clone(),
            ImageViewCreateInfo {
                view_type: ImageViewType::Dim2dArray,
                format: image.format(),
                subresource_range: ImageSubresourceRange {
                    aspects: ImageAspects::COLOR,
                    array_layers: 0..(array.layer_count as u32),
                    mip_levels: 0..array.mip_levels,
                },
                usage: ImageUsage::SAMPLED,
                ..Default::default()
            },
        )
        .unwrap()
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

        info!(what = "updating gpu textures", update_count, invalid_count);
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
}

impl TextureReference {
    pub fn unwrap_indices(&self) -> &'_ TextureStorageIndices {
        match self {
            Self::None => panic!(),
            Self::Managed(storage) => &storage.indices,
        }
    }
}

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

/// Represents a gl texture id.
/// Returned from glGenTextures and used in glBindTexture.
pub type GlTextureId = i32;

#[derive(Debug)]
/// A reference to a minecraft texture. Represents the resource, not the backing texture.
pub struct TextureHandle {
    pub resource_name: Option<String>,
    pub texture_id: GlTextureId,
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
        .unwrap();

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
                    animation: image.get_animation().cloned(),
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
        };

        let temp;

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

#[derive(Derivative)]
#[derivative(Debug)]
pub struct TextureManager {
    #[derivative(Debug = "ignore")]
    allocators: Ref<Allocators>,
    #[derivative(Debug = "ignore")]
    rendering: Ref<RenderManager>,

    #[derivative(Debug = "ignore")]
    pub texture_storage: TextureStorage,

    pub is_resource_pack_reload: bool,
    pub unupdated_textures: HashSet<String>,

    pub textures_by_id: Ref<HashMap<GlTextureId, Arc<TextureHandle>>>,
    pub textures_by_name: Ref<HashMap<String, Arc<TextureHandle>>>,
    pub next_texture_id: GlTextureId,

    pub lookup: Option<Ref<TextureLookup>>,
}

impl TextureManager {
    pub fn new(allocators: &Ref<Allocators>, rendering: &Ref<RenderManager>) -> Self {
        Self {
            allocators: allocators.clone(),
            rendering: rendering.clone(),

            texture_storage: TextureStorage::new(allocators),

            is_resource_pack_reload: false,
            unupdated_textures: HashSet::new(),

            textures_by_id: Ref::new(HashMap::new()),
            textures_by_name: Ref::new(HashMap::new()),
            next_texture_id: 0,

            lookup: None,
        }
    }

    pub fn begin_texture_reload(&mut self) {
        self.is_resource_pack_reload = true;
        self.unupdated_textures = self.textures_by_name.read().keys().cloned().collect();
    }

    pub fn create_texture(&mut self, resource_name: Option<String>) -> Arc<TextureHandle> {
        let id = self.next_texture_id;
        self.next_texture_id += 1;

        let handle = Arc::new(TextureHandle {
            resource_name: resource_name.clone(),
            texture_id: id,
            texture: SpinLock::new(Arc::new(TextureReference::None)),
            animation: None,
            mipmapped: false,
            params: SpinLock::new(TextureParams::default()),
        });

        self.textures_by_id.write().insert(id, handle.clone());

        if let Some(name) = resource_name {
            self.textures_by_name
                .write()
                .insert(name.clone(), handle.clone());
        }

        handle
    }

    pub fn free_texture(&mut self, id: GlTextureId) {
        if let Some(t) = self.textures_by_id.write().remove(&id) {
            if let Some(name) = t.resource_name.as_ref() {
                self.textures_by_name.write().remove(name);
            }
        }
    }

    pub fn get_texture_handle(&self, id: GlTextureId) -> Option<Arc<TextureHandle>> {
        self.textures_by_id.read().get(&id).cloned()
    }

    pub fn enqueue_sprite(
        &mut self,
        name: String,
        uv: [f32; 2],
        image: TextureImage,
    ) -> Result<Arc<TextureHandle>, anyhow::Error> {
        self.unupdated_textures.remove(&name);

        let handle = self.textures_by_name.read().get(&name).cloned();
        let handle = match handle {
            Some(handle) => handle,
            None => self.create_texture(Some(name.clone())),
        };

        let image = image
            .load()
            .with_context(|| format!("could not load image data for texture {name}"))?;

        if matches!(image, TextureImage::None) {
            handle
                .texture
                .set(self.texture_storage.get_missingno().clone());

            return Ok(handle);
        }

        self.texture_storage
            .enqueue_handle_update(&handle, image)
            .with_context(|| format!("could not update gpu texture for texture {name}"))?;

        Ok(handle)
    }

    pub fn finish_texture_reload(&mut self) -> anyhow::Result<()> {
        for skipped in self.unupdated_textures.drain() {
            warn!(
                what = "texture has been skipped in resource reload",
                who = skipped
            );

            let handle = self.textures_by_name.write().remove(&skipped).unwrap();
            self.textures_by_id.write().remove(&handle.texture_id);

            // free the backing texture
            handle
                .texture
                .set(self.texture_storage.get_missingno().clone());
        }

        info!(what = "waiting for all frames to finish for texture reload");

        let mut renderer = self.rendering.write();

        let mut commands = AutoCommandBufferBuilder::primary(
            &self.allocators.read().command_buffer_allocator,
            renderer.queue().queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let pre_record = Instant::now();

        self.texture_storage.record_commands(&mut commands);

        let commands = commands.build()?;

        let post_record = Instant::now();

        renderer.flush()?;

        let fut = commands
            .execute(renderer.queue().clone())?
            .boxed()
            .then_signal_fence_and_flush()?;

        fut.wait(None).unwrap();

        let post_upload = Instant::now();

        info!(
            what = "uploaded all gpu textures",
            count = self.textures_by_name.read().len(),
            record_duration_secs = (post_record - pre_record).as_secs_f32(),
            upload_duration_secs = (post_upload - post_record).as_secs_f32()
        );

        Ok(())
    }

    fn create_lookup(&mut self) {
        todo!()
    }

    pub fn get_lookup(&self) -> Ref<TextureLookup> {
        todo!()
    }
}
