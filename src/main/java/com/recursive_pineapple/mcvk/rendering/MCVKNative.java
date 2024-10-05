package com.recursive_pineapple.mcvk.rendering;

import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.net.URL;
import java.nio.ByteBuffer;
import java.nio.file.Files;

import org.jetbrains.annotations.Nullable;
import org.lwjgl.glfw.GLFW;
import org.lwjgl.opengl.GL;
import org.lwjgl.opengl.GL11;
import org.lwjgl.opengl.GL40;
import org.lwjgl.system.SharedLibrary;

import com.recursive_pineapple.mcvk.MCVK;

import me.eigenraven.lwjgl3ify.api.Lwjgl3Aware;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.data.AnimationMetadataSection;

@Lwjgl3Aware
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

    public static void init(long window_ptr) {
        MCVK.LOG.info("Initializing Vulkan for window 0x" + Long.toHexString(window_ptr));

        SharedLibrary glfw = GLFW.getLibrary();

        MCVKNative.initialize(
            window_ptr,
            glfw.getFunctionAddress("glfwGetRequiredInstanceExtensions"),
            glfw.getFunctionAddress("glfwGetPhysicalDevicePresentationSupport"),
            glfw.getFunctionAddress("glfwCreateWindowSurface"),
            glfw.getFunctionAddress("glfwGetWindowSize")
        );
    }

    public static native void initialize(
        long window_ptr,
        long get_required_instance_extensions,
        long get_physical_device_presentation_support,
        long create_window_surface,
        long get_window_size
    );

    public static native void cleanup();

    public static void setMaxFPS(@Nullable Integer max_fps) {
        setMaxFPS(max_fps == null || max_fps < 0 ? 0 : max_fps);
    }

    /**
     * @param {max_fps} <=0 for no limit, >0 for a limit
     */
    public static native void setMaxFPS(int max_fps);

    public static enum VsyncMode {
        Off(0),
        On(1),
        Triple(2);

        public final int code;

        VsyncMode(int code) {
            this.code = code;
        }
    }

    public static void setVsyncMode(VsyncMode mode) {
        setVsyncMode(mode.code);
    }

    /**
     * @param {mode} 0 = Off, 1 = On, 2 = Triple / Mailbox
     */
    public static native void setVsyncMode(int mode);

    public static native void startFrame(Minecraft mc);

    public static native void finishFrame();

    public static native void enqueueMissingSprite(String name);

    public static native void enqueueFrameSprite(String name, int width, int height, int[][][] frames, String animationJson);

    public static native void enqueueRawSprite(String name, ByteBuffer image, String animationJson);

    public static native void beginTextureReload();

    public static native void finishTextureReload();
}
