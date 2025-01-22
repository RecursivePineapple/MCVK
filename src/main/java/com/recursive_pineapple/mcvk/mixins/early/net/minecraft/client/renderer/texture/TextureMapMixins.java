package com.recursive_pineapple.mcvk.mixins.early.net.minecraft.client.renderer.texture;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.util.List;
import java.util.Map;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Overwrite;
import org.spongepowered.asm.mixin.Shadow;

import com.google.gson.Gson;
import com.recursive_pineapple.mcvk.MCVK;
import com.recursive_pineapple.mcvk.rendering.MCVKNative;
import com.recursive_pineapple.mcvk.utils.IOUtils;

import net.minecraft.client.renderer.texture.TextureAtlasSprite;
import net.minecraft.client.renderer.texture.TextureMap;
import net.minecraft.client.resources.IResource;
import net.minecraft.client.resources.IResourceManager;
import net.minecraft.client.resources.data.AnimationMetadataSection;
import net.minecraft.util.ResourceLocation;
import net.minecraftforge.client.ForgeHooksClient;

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
        this.loadTextureAtlas(resourceManager);
    }

    /**
     * @author Recursive Pineapple
     * @reason The texture map no longer exists
     */
    @Overwrite
    public void loadTextureAtlas(IResourceManager resourceManager) throws IOException {
        MCVK.LOG.info("starting texture atlas reload");

        MCVKNative.beginTextureReload();

        this.registerIcons();

        this.listAnimatedSprites.clear();

        ForgeHooksClient.onTextureStitchedPre((TextureMap)(Object)this);

        cpw.mods.fml.common.ProgressManager.ProgressBar bar = cpw.mods.fml.common.ProgressManager.push("Texture Loading", this.mapRegisteredSprites.size());

        ByteArrayOutputStream staging = new ByteArrayOutputStream(8192);
        byte[] buffer = new byte[8192];

        Gson gson = new Gson();

        // diagonal row tiling
        // we start at 0,0 and lay tiles out along a diagonal line
        // once we hit the end of the line, we go to the next one and reverse direction
        // so it'd be 0,0, then 0,1 then 1,0 then 0,2 then 1,1 then 2,0 etc
        int u = 0, v = 0, du = 1, dv = -1;

        for(var entry : this.mapRegisteredSprites.entrySet()) {
            String name = entry.getKey();
            TextureAtlasSprite sprite = entry.getValue();
            bar.step(name);

            ((TextureAtlasSpriteExt)sprite).setUV(u, v);

            this.loadSprite(gson, resourceManager, name, u, v, sprite, staging, buffer);

            if (u + du < 0) {
                du = -du;
                dv = -dv;
                v++;
            }

            if (v + dv < 0) {
                du = -du;
                dv = -dv;
                u++;
            }

            u += du;
            v += dv;
        }

        MCVKNative.finishTextureReload();

        MCVK.LOG.info("finished texture atlas reload");

        throw new RuntimeException();
    }

    @SuppressWarnings("unchecked")
    private void loadSprite(Gson gson, IResourceManager resourceManager, String name, int u, int v, TextureAtlasSprite sprite, ByteArrayOutputStream staging, byte[] buffer) {
        ResourceLocation location = new ResourceLocation(name);

        if (sprite.hasCustomLoader(resourceManager, location)) {
            if (!sprite.load(resourceManager, location)) {
                List<int[][]> frames;
                AnimationMetadataSection animation;

                try {
                    frames = (List<int[][]>)Statics.framesTextureData.get(sprite);
                    animation = (AnimationMetadataSection)Statics.animationMetadataField.get(sprite);
                } catch (IllegalArgumentException | IllegalAccessException e) {
                    MCVK.LOG.error("could not get sprite frames", e);
                    MCVKNative.enqueueMissingSprite(name, u, v);
                    return;
                }

                int width = sprite.getIconWidth();
                int height = sprite.getIconHeight();

                int[][][] frames_array = frames.toArray(new int[frames.size()][][]);

                MCVKNative.enqueueFrameSprite(
                    name,
                    width, height,
                    frames_array,
                    u, v,
                    gson.toJson(animation)
                );
            }
        } else {
            ResourceLocation spriteLocation = this.completeResourceLocation(location, 0);
            IResource resource;

            try {
                resource = resourceManager.getResource(spriteLocation);
            } catch (IOException e) {
                MCVK.LOG.error("could not get sprite resource", e);
                MCVKNative.enqueueMissingSprite(name);
                return;
            }

            AnimationMetadataSection animation = (AnimationMetadataSection)resource.getMetadata("animation");

            ((TextureAtlasSpriteExt)sprite).setAnimationMetadata(animation);

            InputStream is = resource.getInputStream();

            try {
                staging.reset();

                byte[] data = IOUtils.readStreamToBytes(is, staging, buffer);

                ByteBuffer buffer2 = ByteBuffer.allocateDirect(data.length);
                buffer2.put(data);
                buffer2.flip();

                MCVKNative.enqueueRawSprite(name, buffer2, u, v, gson.toJson(animation));
            } catch (IOException e) {
                MCVK.LOG.error("could not get sprite image data", e);
                MCVKNative.enqueueMissingSprite(name);
                return;
            }
        }
    }
}
