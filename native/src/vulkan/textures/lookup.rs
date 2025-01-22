use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use nalgebra_glm::Vec2;
use static_aabb2d_index::StaticAABB2DIndex;
use static_aabb2d_index::StaticAABB2DIndexBuilder;
use vulkano::image::view::ImageView;

use crate::vulkan::utils::map;
use crate::vulkan::utils::Ref;

use super::texture_manager::ArrayIndex;
use super::texture_manager::ArraySlotIndex;
use super::texture_manager::GlTextureId;
use super::texture_manager::TextureHandle;
use super::texture_manager::TextureManager;
use super::texture_manager::TextureReference;

#[derive(Debug, Clone)]
pub struct TextureAtlasSprite {
    texture: Arc<TextureHandle>,
    u: Vec2,
    v: Vec2,
}

impl TextureAtlasSprite {
    /// Transforms a U,V coordinate from [self.min, self.max] space to [0, 1] space
    pub fn transform(&self, uv: [f32; 2]) -> [f32; 2] {
        [
            map(uv[0], self.u[0], self.u[1], 0f32, 1f32),
            map(uv[1], self.v[0], self.v[1], 0f32, 1f32),
        ]
    }
}

#[derive(Debug)]
struct TextureAtlas {
    texture_id: GlTextureId,
    lookup: StaticAABB2DIndex<f32>,
    sprites: Vec<Arc<TextureAtlasSprite>>,
}

impl TextureAtlas {
    pub fn new(texture_id: GlTextureId, sprites: Vec<Arc<TextureAtlasSprite>>) -> Self {
        let mut builder = StaticAABB2DIndexBuilder::<f32>::new(sprites.len());

        for sprite in &sprites {
            builder.add(sprite.u[0], sprite.v[0], sprite.u[1], sprite.v[1]);
        }

        Self {
            texture_id,
            lookup: builder.build().unwrap(),
            sprites,
        }
    }

    pub fn find(&self, u: f32, v: f32) -> Option<&Arc<TextureAtlasSprite>> {
        let matches = self.lookup.query(u, v, u, v);

        if matches.len() != 1 {
            None
        } else {
            self.sprites.get(matches[0])
        }
    }
}

#[derive(Debug)]
pub struct TextureLookup {
    textures: Arc<TextureManager>,

    blocks: TextureAtlas,
    items: TextureAtlas,
    missingno: TextureAtlasSprite,

    tick_counter: AtomicU32,
}

impl TextureLookup {
    pub fn new(textures: Arc<TextureManager>, blocks: GlTextureId, items: GlTextureId) -> Self {
        Self {
            textures: textures.clone(),
            blocks: TextureAtlas::new(texture_id, sprites),
        }
    }

    fn transform_texture(
        &self,
        sprite: Arc<TextureHandle>,
        uvs: &mut [f32],
    ) -> Option<(
        HashMap<ArrayIndex, Arc<ImageView>>,
        Vec<(ArrayIndex, ArraySlotIndex)>,
    )> {
        let mut textures = HashMap::<ArrayIndex, Arc<ImageView>>::new();
        let mut texture_indices = Vec::<(ArrayIndex, ArraySlotIndex)>::with_capacity(uvs.len());

        let tick = self.tick_counter.load(Ordering::Relaxed);

        let slot_index = match sprite.animation.as_ref() {
            Some(anim) => anim.animation_frames[tick as usize % anim.animation_frames.len()],
            None => 0,
        };

        let storage = sprite.texture.lock();

        let array;
        let slot;

        match Arc::as_ref(&storage) {
            TextureReference::None => {
                let storage = self.missingno.texture.texture.lock();
                array = storage.unwrap_indices().array;
                slot = storage.unwrap_indices().slots[0];
            }
            TextureReference::Managed(storage) => {
                array = storage.indices.array;
                slot = storage.indices.slots[slot_index as usize];
            }
        }

        textures.insert(array, self.textures.texture_storage.get_view(array));

        for _ in 0..(uvs.len() / 2) {
            texture_indices.push((array, slot));
        }

        Some((textures, texture_indices))
    }

    fn transform_atlas_uv(
        &self,
        atlas: &TextureAtlas,
        uvs: &mut [f32],
    ) -> Option<(
        HashMap<ArrayIndex, Arc<ImageView>>,
        Vec<(ArrayIndex, ArraySlotIndex)>,
    )> {
        let mut textures = HashMap::<ArrayIndex, Arc<ImageView>>::new();
        let mut texture_indices = Vec::<(ArrayIndex, ArraySlotIndex)>::with_capacity(uvs.len());

        let tick = self.tick_counter.load(Ordering::Relaxed);

        for vertex in 0..(uvs.len() / 2) {
            let u = uvs[vertex * 2];
            let v = uvs[vertex * 2 + 1];

            let sprite = atlas.find(u, v).unwrap_or(&self.missingno);

            let [u, v] = sprite.transform([u, v]);
            uvs[vertex * 2] = u;
            uvs[vertex * 2 + 1] = v;

            let slot_index = match sprite.texture.animation.as_ref() {
                Some(anim) => anim.animation_frames[tick as usize % anim.animation_frames.len()],
                None => 0,
            };

            let storage = sprite.texture.texture.lock();

            let array;
            let slot;

            match Arc::as_ref(&storage) {
                TextureReference::None => {
                    let storage = self.missingno.texture.texture.lock();
                    array = storage.unwrap_indices().array;
                    slot = storage.unwrap_indices().slots[0];
                }
                TextureReference::Managed(storage) => {
                    array = storage.indices.array;
                    slot = storage.indices.slots[slot_index as usize];
                }
            }

            if !textures.contains_key(&array) {
                textures.insert(array, self.textures.texture_storage.get_view(array));
            }

            texture_indices.push((array, slot));
        }

        Some((textures, texture_indices))
    }

    pub fn transform(
        &self,
        texture: GlTextureId,
        uvs: &mut [f32],
    ) -> Option<(
        HashMap<ArrayIndex, Arc<ImageView>>,
        Vec<(ArrayIndex, ArraySlotIndex)>,
    )> {
        if texture == self.blocks.texture_id {
            return self.transform_atlas_uv(&self.blocks, uvs);
        }

        if texture == self.items.texture_id {
            return self.transform_atlas_uv(&self.items, uvs);
        }

        let sprite = self.textures.textures_by_id.read().get(&texture)?.clone();

        self.transform_texture(sprite, uvs)
    }
}
