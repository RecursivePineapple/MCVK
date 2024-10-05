use std::io::Cursor;

use image::{GenericImageView, ImageError, ImageReader, RgbaImage};
use vulkano::{buffer::AllocateBufferError, image::AllocateImageError, Validated};

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
}

#[derive(Debug, Clone)]
pub struct AnimationMetadata {
    /// one index per tick
    pub animation_frames: Vec<u16>,
}

pub enum TextureImage {
    None,
    Data {
        data: Vec<u8>,
        animation: Option<AnimationMetadata>,
    },
    Static {
        image: RgbaImage,
    },
    Spritesheet {
        sheet: RgbaImage,
        animation: Option<AnimationMetadata>,
    },
    Frames {
        width: u32,
        height: u32,
        frames: Vec<RgbaImage>,
        animation: AnimationMetadata,
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
            ..
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
            Self::Static { image, .. } => image.width(),
            Self::Frames { width, .. } => *width,
            _ => {
                panic!("cannot call width() on an unloaded texture image");
            }
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            Self::Static { image, .. } => image.height(),
            Self::Frames { height, .. } => *height,
            _ => {
                panic!("cannot call height() on an unloaded texture image");
            }
        }
    }

    pub fn get_frames(&self) -> Vec<&RgbaImage> {
        match self {
            TextureImage::Static { image } => vec![image],
            TextureImage::Frames { frames, .. } => frames.iter().collect(),
            _ => {
                panic!("cannot call height() on an unloaded texture image");
            }
        }
    }

    pub fn get_animation(&self) -> Option<&AnimationMetadata> {
        match self {
            TextureImage::None => None,
            TextureImage::Data { animation, .. } => animation.as_ref(),
            TextureImage::Static { .. } => None,
            TextureImage::Spritesheet { animation, .. } => animation.as_ref(),
            TextureImage::Frames { animation, .. } => Some(animation),
        }
    }

    pub fn load(self) -> Result<Self, TextureLoadError> {
        match self {
            Self::Data { data, animation } => {
                let reader = ImageReader::new(Cursor::new(data))
                    .with_guessed_format()
                    .expect("should not fail");

                let image = reader.decode()?;

                let image = image.to_rgba8();

                Self::from_spritesheet(image, animation)
            }
            Self::Spritesheet { sheet, animation } => Self::from_spritesheet(sheet, animation),
            other => Ok(other),
        }
    }

    pub fn from_spritesheet(
        sheet: RgbaImage,
        animation: Option<AnimationMetadata>,
    ) -> Result<Self, TextureLoadError> {
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
                animation: match animation {
                    Some(a) => a,
                    None => AnimationMetadata {
                        animation_frames: (0..frame_count as u16).collect(),
                    },
                },
            })
        } else {
            Ok(Self::Static { image: sheet })
        }
    }
}
