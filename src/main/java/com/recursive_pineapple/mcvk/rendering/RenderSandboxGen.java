package com.recursive_pineapple.mcvk.rendering;

import org.lwjgl.MemoryUtil;
import java.nio.ByteBuffer;
import java.nio.DoubleBuffer;
import java.nio.FloatBuffer;
import java.nio.IntBuffer;
import java.nio.ShortBuffer;

public class RenderSandboxGen {
    public static void glColorPointer(int size, int stride, ByteBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_COLOR, RenderSandbox.ITEM_TYPE_BYTES, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glSecondaryColorPointer(int size, int stride, ByteBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_COLOR_SECONDARY, RenderSandbox.ITEM_TYPE_BYTES, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glIndexPointer(int size, int stride, ByteBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_INDEX, RenderSandbox.ITEM_TYPE_BYTES, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glNormalPointer(int size, int stride, ByteBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_NORMAL, RenderSandbox.ITEM_TYPE_BYTES, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glTexCoordPointer(int size, int stride, ByteBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_TEXCOORD, RenderSandbox.ITEM_TYPE_BYTES, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glVertexPointer(int size, int stride, ByteBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_VERTEX, RenderSandbox.ITEM_TYPE_BYTES, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glColorPointer(int size, int stride, ShortBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_COLOR, RenderSandbox.ITEM_TYPE_SHORTS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glSecondaryColorPointer(int size, int stride, ShortBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_COLOR_SECONDARY, RenderSandbox.ITEM_TYPE_SHORTS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glIndexPointer(int size, int stride, ShortBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_INDEX, RenderSandbox.ITEM_TYPE_SHORTS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glNormalPointer(int size, int stride, ShortBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_NORMAL, RenderSandbox.ITEM_TYPE_SHORTS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glTexCoordPointer(int size, int stride, ShortBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_TEXCOORD, RenderSandbox.ITEM_TYPE_SHORTS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glVertexPointer(int size, int stride, ShortBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_VERTEX, RenderSandbox.ITEM_TYPE_SHORTS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glColorPointer(int size, int stride, IntBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_COLOR, RenderSandbox.ITEM_TYPE_INTS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glSecondaryColorPointer(int size, int stride, IntBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_COLOR_SECONDARY, RenderSandbox.ITEM_TYPE_INTS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glIndexPointer(int size, int stride, IntBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_INDEX, RenderSandbox.ITEM_TYPE_INTS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glNormalPointer(int size, int stride, IntBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_NORMAL, RenderSandbox.ITEM_TYPE_INTS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glTexCoordPointer(int size, int stride, IntBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_TEXCOORD, RenderSandbox.ITEM_TYPE_INTS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glVertexPointer(int size, int stride, IntBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_VERTEX, RenderSandbox.ITEM_TYPE_INTS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glColorPointer(int size, int stride, FloatBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_COLOR, RenderSandbox.ITEM_TYPE_FLOATS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glSecondaryColorPointer(int size, int stride, FloatBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_COLOR_SECONDARY, RenderSandbox.ITEM_TYPE_FLOATS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glIndexPointer(int size, int stride, FloatBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_INDEX, RenderSandbox.ITEM_TYPE_FLOATS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glNormalPointer(int size, int stride, FloatBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_NORMAL, RenderSandbox.ITEM_TYPE_FLOATS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glTexCoordPointer(int size, int stride, FloatBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_TEXCOORD, RenderSandbox.ITEM_TYPE_FLOATS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glVertexPointer(int size, int stride, FloatBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_VERTEX, RenderSandbox.ITEM_TYPE_FLOATS, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glColorPointer(int size, int stride, DoubleBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_COLOR, RenderSandbox.ITEM_TYPE_DOUBLES, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glSecondaryColorPointer(int size, int stride, DoubleBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_COLOR_SECONDARY, RenderSandbox.ITEM_TYPE_DOUBLES, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glIndexPointer(int size, int stride, DoubleBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_INDEX, RenderSandbox.ITEM_TYPE_DOUBLES, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glNormalPointer(int size, int stride, DoubleBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_NORMAL, RenderSandbox.ITEM_TYPE_DOUBLES, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glTexCoordPointer(int size, int stride, DoubleBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_TEXCOORD, RenderSandbox.ITEM_TYPE_DOUBLES, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
    public static void glVertexPointer(int size, int stride, DoubleBuffer pointer) {
        RenderSandbox.addPointerArray(size, stride, RenderSandbox.ARRAY_TYPE_VERTEX, RenderSandbox.ITEM_TYPE_DOUBLES, MemoryUtil.getAddress(pointer), pointer.remaining());
    }
}
