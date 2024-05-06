package com.recursive_pineapple.mcvk.mixins.early.net.minecraft.client.renderer.texture;

import java.lang.reflect.Field;

import net.minecraft.client.renderer.texture.TextureAtlasSprite;

public class Statics {
    
    public static final Field framesTextureData;
    static {
        try {
            framesTextureData = TextureAtlasSprite.class.getField("framesTextureData");
        } catch (NoSuchFieldException | SecurityException e) {
            throw new RuntimeException(e);
        }
        framesTextureData.setAccessible(true);
    }

}
