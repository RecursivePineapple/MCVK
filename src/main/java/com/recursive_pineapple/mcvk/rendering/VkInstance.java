package com.recursive_pineapple.mcvk.rendering;

import static org.lwjgl.glfw.GLFW.GLFW_CLIENT_API;
import static org.lwjgl.glfw.GLFW.GLFW_NO_API;
import static org.lwjgl.glfw.GLFW.glfwWindowHint;

import java.lang.reflect.Field;
import java.nio.Buffer;
import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.List;
import java.util.logging.Logger;

import org.lwjgl.PointerBuffer;
import org.lwjgl.glfw.GLFW;
import org.lwjgl.system.SharedLibrary;

import com.recursive_pineapple.mcvk.MCVK;

import me.eigenraven.lwjgl3ify.api.Lwjgl3Aware;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.data.AnimationMetadataSection;
import sun.misc.Unsafe;

@Lwjgl3Aware
public class VkInstance {
    
    private static VkInstance INSTANCE;

    public static void init(long window_ptr) {
        if(INSTANCE != null) {
            throw new IllegalStateException("cannot call VkInstance.init() multiple times");
        }

        INSTANCE = new VkInstance(window_ptr);
    }

    public static VkInstance getInstance() {
        if(INSTANCE == null) {
            throw new IllegalStateException("cannot call VkInstance.getInstance() before VkInstance.init() has been called");
        }

        return INSTANCE;
    }

    private long instance_ptr;

    VkInstance(long window_ptr) {
        MCVK.LOG.info("Initializing Vulkan for window 0x" + Long.toHexString(window_ptr));

        SharedLibrary glfw = GLFW.getLibrary();

        this.instance_ptr = MCVKNative.createInstance(
            window_ptr,
            glfw.getFunctionAddress("glfwGetRequiredInstanceExtensions"),
            glfw.getFunctionAddress("glfwGetPhysicalDevicePresentationSupport"),
            glfw.getFunctionAddress("glfwCreateWindowSurface"),
            glfw.getFunctionAddress("glfwGetWindowSize")
        );
    }

    public long getInstancePointer() {
        return instance_ptr;
    }

    @Override
    protected final void finalize() throws Throwable {
        super.finalize();

        this.cleanup();
    }

    public void cleanup() {
        
        if(instance_ptr != 0) {
            MCVKNative.destroyInstance(instance_ptr);
            instance_ptr = 0;
        }
    }

    public static enum WindowMode {
        Windowed(0),
        BorderlessWindow(1),
        ExclusiveFullscreen(2);

        public final int code;

        WindowMode(int code) {
            this.code = code;
        }
    }

    public void setMaxFPS(Integer max_fps) {
        MCVKNative.setMaxFPS(instance_ptr, max_fps == null || max_fps < 0 ? 0 : max_fps);
    }

    public static enum VsyncMode {
        Off(0),
        On(1),
        Triple(2);

        public final int code;

        VsyncMode(int code) {
            this.code = code;
        }
    }

    public void setVsyncMode(VsyncMode mode) {
        MCVKNative.setVsyncMode(instance_ptr, mode.code);
    }

    public void startFrame() {
        MCVKNative.startFrame(instance_ptr, Minecraft.getMinecraft());
    }

    public void finishFrame() {
        MCVKNative.finishFrame(instance_ptr);
    }

    public void enqueueMissingSprite(String name) {
        MCVKNative.enqueueMissingSprite(instance_ptr, name);
    }

    public void enqueueFrameSprite(String name, int width, int height, int[][][] frames, AnimationMetadataSection animation) {
        MCVKNative.enqueueFrameSprite(instance_ptr, name, width, height, frames, animation);
    }

    public void enqueueRawSprite(String name, ByteBuffer image, AnimationMetadataSection animation) {
        MCVKNative.enqueueRawSprite(instance_ptr, name, image, animation);
    }

    public void loadTextures(int max_mipmap_levels, boolean gen_aniso_data) {
        MCVKNative.loadTextures(instance_ptr, max_mipmap_levels, gen_aniso_data);
    }
}
