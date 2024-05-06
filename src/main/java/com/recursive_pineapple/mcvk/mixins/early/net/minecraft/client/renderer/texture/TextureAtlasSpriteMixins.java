package com.recursive_pineapple.mcvk.mixins.early.net.minecraft.client.renderer.texture;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;

import net.minecraft.client.renderer.texture.TextureAtlasSprite;
import net.minecraft.client.resources.data.AnimationMetadataSection;

@Mixin(TextureAtlasSprite.class)
public class TextureAtlasSpriteMixins implements TextureAtlasSpriteExt {
    @Shadow
    private AnimationMetadataSection animationMetadata;

    @Override
    public void setAnimationMetadata(AnimationMetadataSection animationMetadata) {
        this.animationMetadata = animationMetadata;
    }
}
