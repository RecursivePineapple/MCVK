use std::{
    io::Cursor,
    mem::size_of,
    sync::{atomic::AtomicU8, Arc},
};

use image::{io::Reader, GenericImageView, ImageError, RgbaImage};
use vulkano::{
    buffer::{AllocateBufferError, BufferUsage},
    command_buffer::{
        AutoCommandBufferBuilder, BlitImageInfo, CopyBufferToImageInfo, PrimaryAutoCommandBuffer,
    },
    image::{AllocateImageError, Image, ImageLayout, ImageUsage},
    memory::allocator::StandardMemoryAllocator,
    Validated, ValidationError,
};

use crate::vulkan::spinlock::SpinLock;

#[derive(Debug, Clone)]
pub struct TextureHandle {
    pub handle: Arc<TextureHandleData>,
}

#[derive(Debug)]
pub struct TextureHandleData {
    pub texture: SpinLock<Arc<Texture>>,
}

impl PartialEq for TextureHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.handle, &other.handle)
    }
}

impl TextureHandle {
    pub fn new(texture: Arc<Texture>) -> Self {
        Self {
            handle: Arc::new(TextureHandleData {
                texture: SpinLock::new(texture),
            }),
        }
    }

    pub fn replace(&self, new_texture: Arc<Texture>) {
        *self.handle.texture.lock() = new_texture;
    }
}

#[derive(Debug, Clone)]
pub struct AnimationMetadata {
    pub animation_frames: Vec<usize>,
    pub frame_width: usize,
    pub frame_height: usize,
    pub frame_time: usize,
}

#[derive(Debug)]
pub enum Texture {
    None,
    Individual {
        texture: Arc<Image>,
        mipmap_levels: u32,
        animation: Option<AnimationMetadata>,
        // has_anisotropic_data: bool,
    },
    // Dynamic {
    //     textures: Vec<Arc<Image>>,
    //     active: AtomicU8,
    //     mipmap_levels: u32,
    //     animation: Option<AnimationMetadata>,
    //     // has_anisotropic_data: bool,
    // },
}

#[derive(Debug, thiserror::Error)]
pub enum TextureLoadError {
    #[error("image data was incorrect: {0}")]
    BadImageBytes(#[from] ImageError),
    #[error("image was not square and the bigger side was not an integer multiple of the smaller side (width = {0}, height = {1}, animation frame = {2:?})")]
    BadImageSize(u32, u32, Option<u32>),
    // #[error("could not generate mipmaps: a dimension of the image was not a power of 2 (width = {0}, height = {1}, animation frame = {2:?})")]
    // ImageNotPow2(u32, u32, Option<u32>),
    #[error("could not create vulkan image: {0}")]
    ImageError(#[from] Validated<AllocateImageError>),
    #[error("could not create source buffer: {0}")]
    BufferCreateError(#[from] Validated<AllocateBufferError>),
    #[error("could not generate mipmap level {0}: {1}")]
    MipMapGenError(u32, Box<ValidationError>),
}

// fn round_up_to_power_of_two(x: u32) -> u32 {
//     let mut y = x - 1;
//     y |= y >> 1;
//     y |= y >> 2;
//     y |= y >> 4;
//     y |= y >> 8;
//     y |= y >> 16;
//     y + 1
// }

// fn is_power_of_two(x: u32) -> bool {
//     x != 0 && (x & (x - 1)) == 0
// }

pub enum TextureImage {
    Static {
        image: RgbaImage,
    },
    Frames {
        width: u32,
        height: u32,
        frames: Vec<RgbaImage>,
    },
}

impl TextureImage {
    pub fn validate(&self) -> Result<(), TextureLoadError> {
        if self.width() != self.height() {
            return Err(TextureLoadError::BadImageSize(
                self.width(),
                self.height(),
                None,
            ));
        }

        if let Self::Frames {
            width,
            height,
            frames,
        } = self
        {
            for (i, frame) in frames.iter().enumerate() {
                if frame.width() != *width || frame.height() != *height {
                    return Err(TextureLoadError::BadImageSize(
                        *width,
                        *height,
                        Some(i as u32),
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn width(&self) -> u32 {
        match self {
            TextureImage::Static { image } => image.width(),
            TextureImage::Frames { width, .. } => *width,
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            TextureImage::Static { image } => image.height(),
            TextureImage::Frames { height, .. } => *height,
        }
    }

    // pub fn frame_count(&self) -> usize {
    //     match self {
    //         TextureImage::Static { .. } => 1,
    //         TextureImage::Frames { frames, .. } => frames.len(),
    //     }
    // }

    pub fn max_mipmap_levels(&self) -> u32 {
        (size_of::<usize>() * 8) as u32 - (self.width().leading_zeros())
    }

    pub fn from_spritesheet(sheet: RgbaImage) -> Result<Self, TextureLoadError> {
        let width = sheet.width();
        let height = sheet.height();

        if width != height {
            if height % width != 0 && width % height != 0 {
                return Err(TextureLoadError::BadImageSize(width, height, None));
            }

            let sprite_size = width.min(height);

            let frame_count = width.max(height) / sprite_size;

            let mut frames = Vec::new();

            let stride = if height > width {
                (0, sprite_size)
            } else {
                (sprite_size, 0)
            };

            for i in 0..frame_count {
                frames.push(
                    sheet
                        .view(stride.0 * i, stride.1 * i, sprite_size, sprite_size)
                        .to_image(),
                );
            }

            Ok(Self::Frames {
                width,
                height,
                frames,
            })
        } else {
            Ok(Self::Static { image: sheet })
        }
    }

    pub fn get_frames(&self) -> Vec<&RgbaImage> {
        match self {
            TextureImage::Static { image } => vec![image],
            TextureImage::Frames { frames, .. } => frames.iter().collect(),
        }
    }
}

pub fn load_from_data(
    allocator: Arc<StandardMemoryAllocator>,
    commands: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    data: &[u8],
    max_mipmap_levels: u32,
    gen_aniso_data: bool,
    animation: Option<AnimationMetadata>,
) -> Result<Texture, TextureLoadError> {
    let reader = Reader::new(Cursor::new(data))
        .with_guessed_format()
        .expect("should not fail");

    let image = reader.decode()?;

    let image = image.to_rgba8();

    load_from_image(
        allocator,
        commands,
        TextureImage::Static { image },
        max_mipmap_levels,
        gen_aniso_data,
        animation,
    )
}

pub fn load_from_image(
    allocator: Arc<StandardMemoryAllocator>,
    commands: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    image: TextureImage,
    max_mipmap_levels: u32,
    gen_aniso_data: bool,
    animation: Option<AnimationMetadata>,
) -> Result<Texture, TextureLoadError> {
    image.validate()?;

    let width = image.width();
    let height = image.height();
    let frames = image.get_frames();
    let mipmap_levels = max_mipmap_levels.min(image.max_mipmap_levels());

    // TODO: this
    let _ = gen_aniso_data;

    let frame_pixel_size = (image.width() * image.height()) as usize;

    let mut image_data = Vec::with_capacity(frame_pixel_size * frames.len());

    for frame in &frames {
        for pixel in frame.pixels() {
            let [r, g, b, a] = pixel.0.clone();
            image_data.push(u32::from_ne_bytes([a, b, g, r]));
        }
    }

    let texture = Image::new(
        allocator.clone(),
        vulkano::image::ImageCreateInfo {
            extent: [width, height, 1],
            array_layers: frames.len() as u32,
            usage: ImageUsage::SAMPLED | ImageUsage::TRANSFER_SRC | ImageUsage::TRANSFER_DST,
            format: vulkano::format::Format::A8B8G8R8_UINT_PACK32,
            initial_layout: ImageLayout::Undefined,
            image_type: vulkano::image::ImageType::Dim2d,
            mip_levels: mipmap_levels + 1,
            ..Default::default()
        },
        vulkano::memory::allocator::AllocationCreateInfo {
            ..Default::default()
        },
    )?;

    let source_buffer = vulkano::buffer::Buffer::new_slice::<u32>(
        allocator.clone(),
        vulkano::buffer::BufferCreateInfo {
            usage: BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        vulkano::memory::allocator::AllocationCreateInfo {
            ..Default::default()
        },
        image_data.len() as u64,
    )?;

    {
        let mut guard = source_buffer.write().unwrap();
        guard.copy_from_slice(&image_data);
    }

    let mut copy = CopyBufferToImageInfo::buffer_image(source_buffer, texture.clone());
    copy.dst_image_layout = ImageLayout::TransferSrcOptimal;
    let region = copy.regions[0].clone();
    copy.regions = (0..frames.len())
        .map(|frame| {
            let mut region = region.clone();

            region.buffer_offset = (frame * frame_pixel_size * 4) as u64;
            region.image_subresource.array_layers = (frame as u32)..(frame as u32) + 1;

            region
        })
        .collect();

    commands
        .copy_buffer_to_image(copy)
        .map_err(|e| TextureLoadError::MipMapGenError(0, e))?;

    for i in 1..mipmap_levels + 1 {
        let mut blit = BlitImageInfo::images(texture.clone(), texture.clone());

        let region = &mut blit.regions[0];

        region.src_subresource.mip_level = i - 1;
        region.src_offsets[1] = [image.width() >> (i - 1), image.height() >> (i - 1), 1];

        region.dst_subresource.mip_level = i;
        region.dst_offsets[1] = [image.width() >> i, image.height() >> i, 1];

        commands
            .blit_image(blit)
            .map_err(|e| TextureLoadError::MipMapGenError(i, e))?;
    }

    Ok(Texture::Individual {
        texture: texture,
        mipmap_levels,
        animation,
        // has_anisotropic_data: (),
    })
}
