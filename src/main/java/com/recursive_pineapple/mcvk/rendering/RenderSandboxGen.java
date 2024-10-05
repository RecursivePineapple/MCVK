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
    public static native void glVertex2f(float x, float y);
    public static native void glVertex2d(double x, double y);
    public static native void glVertex2i(int x, int y);
    public static native void glVertex2s(short x, short y);
    public static native void glVertex2b(byte x, byte y);
    public static native void glVertex2l(long x, long y);
    public static native void glVertex3f(float x, float y, float z);
    public static native void glVertex3d(double x, double y, double z);
    public static native void glVertex3i(int x, int y, int z);
    public static native void glVertex3s(short x, short y, short z);
    public static native void glVertex3b(byte x, byte y, byte z);
    public static native void glVertex3l(long x, long y, long z);
    public static native void glVertex4f(float x, float y, float z, float w);
    public static native void glVertex4d(double x, double y, double z, double w);
    public static native void glVertex4i(int x, int y, int z, int w);
    public static native void glVertex4s(short x, short y, short z, short w);
    public static native void glVertex4b(byte x, byte y, byte z, byte w);
    public static native void glVertex4l(long x, long y, long z, long w);
    public static native void glTexCoord2f(float x, float y);
    public static native void glTexCoord2d(double x, double y);
    public static native void glTexCoord2i(int x, int y);
    public static native void glTexCoord2s(short x, short y);
    public static native void glTexCoord2b(byte x, byte y);
    public static native void glTexCoord2l(long x, long y);
    public static native void glTexCoord3f(float x, float y, float z);
    public static native void glTexCoord3d(double x, double y, double z);
    public static native void glTexCoord3i(int x, int y, int z);
    public static native void glTexCoord3s(short x, short y, short z);
    public static native void glTexCoord3b(byte x, byte y, byte z);
    public static native void glTexCoord3l(long x, long y, long z);
    public static native void glTexCoord4f(float x, float y, float z, float w);
    public static native void glTexCoord4d(double x, double y, double z, double w);
    public static native void glTexCoord4i(int x, int y, int z, int w);
    public static native void glTexCoord4s(short x, short y, short z, short w);
    public static native void glTexCoord4b(byte x, byte y, byte z, byte w);
    public static native void glTexCoord4l(long x, long y, long z, long w);
    public static native void glNormal2f(float x, float y);
    public static native void glNormal2d(double x, double y);
    public static native void glNormal2i(int x, int y);
    public static native void glNormal2s(short x, short y);
    public static native void glNormal2b(byte x, byte y);
    public static native void glNormal2l(long x, long y);
    public static native void glNormal3f(float x, float y, float z);
    public static native void glNormal3d(double x, double y, double z);
    public static native void glNormal3i(int x, int y, int z);
    public static native void glNormal3s(short x, short y, short z);
    public static native void glNormal3b(byte x, byte y, byte z);
    public static native void glNormal3l(long x, long y, long z);
    public static native void glNormal4f(float x, float y, float z, float w);
    public static native void glNormal4d(double x, double y, double z, double w);
    public static native void glNormal4i(int x, int y, int z, int w);
    public static native void glNormal4s(short x, short y, short z, short w);
    public static native void glNormal4b(byte x, byte y, byte z, byte w);
    public static native void glNormal4l(long x, long y, long z, long w);
    public static native void glColor3f(float x, float y, float z);
    public static native void glColor3d(double x, double y, double z);
    public static native void glColor3i(int x, int y, int z);
    public static native void glColor3s(short x, short y, short z);
    public static native void glColor3b(byte x, byte y, byte z);
    public static native void glColor3l(long x, long y, long z);
    public static native void glColor4f(float x, float y, float z, float w);
    public static native void glColor4d(double x, double y, double z, double w);
    public static native void glColor4i(int x, int y, int z, int w);
    public static native void glColor4s(short x, short y, short z, short w);
    public static native void glColor4b(byte x, byte y, byte z, byte w);
    public static native void glColor4l(long x, long y, long z, long w);
}
