package com.recursive_pineapple.mcvk.mixins.early.net.minecraft.client.renderer.texture;

import java.lang.reflect.Field;

import net.minecraft.client.renderer.texture.TextureAtlasSprite;

public class Statics {
    public static final Field
        framesTextureData,
        animationMetadataField;

    static {
        try {
            framesTextureData = TextureAtlasSprite.class.getField("framesTextureData");
            framesTextureData.setAccessible(true);

            animationMetadataField = TextureAtlasSprite.class.getField("animationMetadata");
            animationMetadataField.setAccessible(true);
        } catch (NoSuchFieldException | SecurityException e) {
            throw new RuntimeException(e);
        }
    }
}
