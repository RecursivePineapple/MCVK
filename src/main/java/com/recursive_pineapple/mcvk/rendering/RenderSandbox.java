package com.recursive_pineapple.mcvk.rendering;

import java.nio.ByteBuffer;
import java.nio.DoubleBuffer;
import java.nio.FloatBuffer;
import java.nio.IntBuffer;
import java.nio.ShortBuffer;

import org.lwjgl.MemoryUtil;

public class RenderSandbox {
    
    private RenderSandbox() {

    }

    public native static void glMatrixMode(int mode);

    public native static void glPushMatrix();

    public native static void glPopMatrix();

    public native static void glLoadIdentity();

    public native static void glOrtho(double left, double right, double bottom, double top, double zNear, double zFar);

    public native static void glTranslatef(float x, float y, float z);

    public native static void glEnable(int cap);

    public native static void glDisable(int cap);

    public native static void glEnableClientState(int cap);

    public static final int ARRAY_TYPE_COLOR = 0;
    public static final int ARRAY_TYPE_COLOR_SECONDARY = 1;
    public static final int ARRAY_TYPE_INDEX = 2;
    public static final int ARRAY_TYPE_NORMAL = 3;
    public static final int ARRAY_TYPE_TEXCOORD = 4;
    public static final int ARRAY_TYPE_VERTEX = 5;

    public static final int ITEM_TYPE_BYTES = 0;
    public static final int ITEM_TYPE_SHORTS = 1;
    public static final int ITEM_TYPE_INTS = 2;
    public static final int ITEM_TYPE_FLOATS = 3;
    public static final int ITEM_TYPE_DOUBLES = 4;

    public static native void addPointerArray(int size, int stride, int array_type, int item_type, long start, int item_count);

    public native static void glDrawArrays(int mode, int first, int count);

}
