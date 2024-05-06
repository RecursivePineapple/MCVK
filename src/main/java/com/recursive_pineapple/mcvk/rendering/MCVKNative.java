package com.recursive_pineapple.mcvk.rendering;

import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.net.URL;
import java.nio.ByteBuffer;
import java.nio.file.Files;

import com.recursive_pineapple.mcvk.MCVK;

import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.data.AnimationMetadataSection;

public class MCVKNative {

    static {
        try {
            String libName = "libmcvk.so";

            URL url = MCVKNative.class.getResource("/natives/" + libName);

            File tempDir = Files.createTempDirectory("mcvk")
                .toFile();
            tempDir.deleteOnExit();

            File tempLib = new File(tempDir, libName);
            tempLib.deleteOnExit();

            try (InputStream in = url.openStream()) {
                Files.copy(in, tempLib.toPath());
            }

            System.load(tempLib.getAbsolutePath());
        } catch (IOException e) {
            MCVK.LOG.error("Could not load the native library for MCVK", e);
            MCVK.setInvalid();
        }
    }

    public static void load() {

    }

    public static native long createInstance(
        long window_ptr,
        long get_required_instance_extensions,
        long get_physical_device_presentation_support,
        long create_window_surface,
        long get_window_size
    );

    public static native void destroyInstance(long ptr);

    /**
     * @param {max_fps} <=0 for no limit, >0 for a limit
     */
    public static native void setMaxFPS(long ptr, int max_fps);

    /**
     * @param {mode} 0 = Off, 1 = On, 2 = Triple / Mailbox
     */
    public static native void setVsyncMode(long ptr, int mode);

    public static native void startFrame(long ptr, Minecraft mc);

    public static native void finishFrame(long ptr);

    public static native void enqueueMissingSprite(long ptr, String name);

    public static native void enqueueFrameSprite(long ptr, String name, int width, int height, int[][][] frames, AnimationMetadataSection animation);

    public static native void enqueueRawSprite(long ptr, String name, ByteBuffer image, AnimationMetadataSection animation);

    public static native void loadTextures(long ptr, int max_mipmap_levels, boolean gen_aniso_data);

}
