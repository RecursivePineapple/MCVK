package com.recursive_pineapple.mcvk.mixins.early.net.minecraft.client.renderer.texture;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;

import net.minecraft.client.renderer.texture.TextureAtlasSprite;
import net.minecraft.client.resources.data.AnimationMetadataSection;

@Mixin(TextureAtlasSprite.class)
public class TextureAtlasSpriteMixins implements TextureAtlasSpriteExt {
    @Shadow
    private AnimationMetadataSection animationMetadata;

    @Shadow
    protected int originX;
    @Shadow
    protected int originY;
    @Shadow
    protected int width;
    @Shadow
    protected int height;
    @Shadow
    private float minU;
    @Shadow
    private float maxU;
    @Shadow
    private float minV;
    @Shadow
    private float maxV;

    @Override
    public void setAnimationMetadata(AnimationMetadataSection animationMetadata) {
        this.animationMetadata = animationMetadata;
    }

    @Override
    public void setUV(int u, int v) {
        originX = u;
        originY = v;
        width = 1;
        height = 1;
        minU = u;
        maxU = u + 1;
        minV = v;
        maxV = v + 1;
    }
}
