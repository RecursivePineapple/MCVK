package com.recursive_pineapple.mcvk.mixins.early.net.minecraft.client.renderer.texture;

import java.io.ByteArrayOutputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.lang.reflect.Field;
import java.nio.ByteBuffer;
import java.nio.charset.Charset;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Overwrite;
import org.spongepowered.asm.mixin.Shadow;

import com.recursive_pineapple.mcvk.MCVK;
import com.recursive_pineapple.mcvk.rendering.VkInstance;

import net.minecraft.client.renderer.texture.TextureAtlasSprite;
import net.minecraft.client.renderer.texture.TextureMap;
import net.minecraft.client.resources.IResource;
import net.minecraft.client.resources.IResourceManager;
import net.minecraft.client.resources.data.AnimationMetadataSection;
import net.minecraft.client.resources.data.TextureMetadataSection;
import net.minecraft.util.ResourceLocation;

@Mixin(TextureMap.class)
public abstract class TextureMapMixins {
    
    @Shadow
    List<TextureAtlasSprite> listAnimatedSprites;
    @Shadow
    Map<String, TextureAtlasSprite> mapRegisteredSprites;

    @Shadow
    abstract void registerIcons();

    @Shadow
    abstract ResourceLocation completeResourceLocation(ResourceLocation location, int mipLevel);

    /**
     * @author Recursive Pineapple
     * @reason The texture map no longer exists
     */
    @Overwrite
    public void loadTexture(IResourceManager resourceManager) throws IOException {
        this.registerIcons();

        this.listAnimatedSprites.clear();

        // ForgeHooksClient.onTextureStitchedPre(this); todo: warn on whatever is in here

        for(var entry : this.mapRegisteredSprites.entrySet()) {
            String name = entry.getKey();
            TextureAtlasSprite sprite = entry.getValue();
            // bar.step(spriteLocation.getResourcePath());

            this.loadSprite(resourceManager, name, sprite);
        }

        VkInstance.getInstance().loadTextures(4, false);
    }

    private void loadSprite(IResourceManager resourceManager, String name, TextureAtlasSprite sprite) {
        ResourceLocation location = new ResourceLocation(name);

        if (sprite.hasCustomLoader(resourceManager, location)) {
            if (!sprite.load(resourceManager, location)) {
                List<int[][]> frames;
                try {
                    frames = (List<int[][]>)Statics.framesTextureData.get(sprite);
                } catch (IllegalArgumentException | IllegalAccessException e) {
                    MCVK.LOG.error("could not get sprite frames", e);
                    VkInstance.getInstance().enqueueMissingSprite(name);
                    return;
                }

                int width = sprite.getIconWidth();
                int height = sprite.getIconHeight();

                int[][][] frames_array = frames.toArray(new int[frames.size()][][]);

                VkInstance.getInstance().enqueueFrameSprite(name, width, height, frames_array, null);
            }
        } else {
            ResourceLocation spriteLocation = this.completeResourceLocation(location, 0);
            IResource resource;

            try {
                resource = resourceManager.getResource(spriteLocation);
            } catch (IOException e) {
                MCVK.LOG.error("could not get sprite resource", e);
                VkInstance.getInstance().enqueueMissingSprite(name);
                return;
            }

            // TextureMetadataSection textureMetadata = (TextureMetadataSection)resource.getMetadata("texture");
            AnimationMetadataSection animationMetadata = (AnimationMetadataSection)resource.getMetadata("animation");

            ((TextureAtlasSpriteExt)sprite).setAnimationMetadata(animationMetadata);

            InputStream is = resource.getInputStream();
            ByteArrayOutputStream staging = new ByteArrayOutputStream(8192);

            try {
                byte[] buffer = new byte[8192];
                int len;
                while ((len = is.read(buffer)) != -1) {
                    staging.write(buffer, 0, len);
                }

                byte[] out = staging.toByteArray();

                ByteBuffer buffer2 = ByteBuffer.allocateDirect(out.length);
                buffer2.put(out);
                buffer2.flip();

                VkInstance.getInstance().enqueueRawSprite(name, buffer2, animationMetadata);;
            } catch (IOException e) {
                MCVK.LOG.error("could not get sprite image data", e);
                VkInstance.getInstance().enqueueMissingSprite(name);
                return;
            }
        }
    }
}
