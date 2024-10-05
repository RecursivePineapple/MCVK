package com.recursive_pineapple.mcvk.rendering;

import java.nio.ByteBuffer;
import java.nio.charset.Charset;

public class RenderSandbox {
    
    private RenderSandbox() {

    }

    public native static void glMatrixMode(int mode);

    public native static void glPushMatrix();

    public native static void glPopMatrix();

    public native static void glLoadIdentity();

    public native static void glOrtho(double left, double right, double bottom, double top, double zNear, double zFar);

    public native static void glTranslatef(float x, float y, float z);
    public native static void glTranslated(double x, double y, double z);

    public native static void glRotatef(float angle, float x, float y, float z);
    public native static void glRotated(double angle, double x, double y, double z);

    public native static void glScalef(float x, float y, float z);
    public native static void glScaled(double x, double y, double z);

    public native static void glEnable(int cap);

    public native static void glDisable(int cap);

    public native static void glEnableClientState(int cap);
    public native static void glDisableClientState(int cap);

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

    public static void glShadeModel(int mode) {
        // TODO: this
    }

    public static void glClearDepth(double value) {
        // TODO?
    }

    public native static void glClear(int mask);

    public static void glClearColor(float r, float g, float b, float a) {
        // TODO: this
    }

    public static void glDepthFunc(int func) {
        // TODO: this
    }

    public static void glDepthMask(boolean flag) {
        // TODO: this
    }

    public static void glAlphaFunc(int func, float ref) {
        // TODO: this
    }

    public static void glBlendFunc(int sfactor, int dfactor) {
        // TODO: this
    }

    public static void glCullFace(int func) {
        // TODO: this
    }

    public static void glViewport(int a, int b, int c, int d) { /* NO-OP */ }

    public static void glColor4f(float r, float g, float b, float a) {
        // TODO: this
    }

    public static void glColorMask(boolean r, boolean g, boolean b, boolean a) {
        // TODO: this
    }

    public static void glFlush() { /* NO-OP? */ }

    public static int glGetError() {
        return 0; // TODO?
    }

    public static void glLineWidth(float width) {
        // TODO: this
    }

    public static int glGetInteger(int param) {
        return 0;
    }

    public static float glGetFloat(int param) {
        return 0f;
    }

    public static String glGetString(int param) {
        return "";
    }

    public static int glGetProgrami(int program, int param) {
        return 0;
    }

    public static void glAttachShader(int program, int shader) {

    }

    public static void glDeleteShader(int shader) { /* NO-OP */ }

    public static int glCreateShader(int type) {
        return 0;
    }

    public static void glShaderSource(int shader, ByteBuffer source) {
        
    }

    public static void glCompileShader(int shader) {

    }

    public static int glGetShaderi(int shader, int param) {
        return 0;
    }

    public static String glGetShaderInfoLog(int shader, int maxLength) {
        return "";
    }

    public static String glGetProgramInfoLog(int program, int maxLength) {
        return "";
    }

    public static void glUseProgram(int program) {

    }

    public static int glCreateProgram() {
        return 0;
    }

    public static void glDeleteProgram(int program) {

    }

    public static void glLinkProgram(int program) {

    }

    private static ByteBuffer seqToBuffer(CharSequence seq) {
        byte[] bytes = seq.toString().getBytes(Charset.forName("UTF8"));

        ByteBuffer buffer = ByteBuffer.allocateDirect(bytes.length);
        buffer.put(bytes);
        buffer.flip();

        return buffer;
    }

    public static int glGetUniformLocation(int program, CharSequence name) {
        return glGetUniformLocation(program, seqToBuffer(name));
    }

    public static int glGetUniformLocation(int program, ByteBuffer name) {
        return 0;
    }

    public static int glGetAttribLocation(int program, CharSequence name) {
        return glGetAttribLocation(program, seqToBuffer(name));
    }

    public static int glGetAttribLocation(int program, ByteBuffer name) {
        return 0;
    }

    public static void glBindFramebuffer(int a, int b) { /* NO-OP */ }

    public static void glBindRenderbuffer(int a, int b) { /* NO-OP */ }

    public static void glDeleteRenderbuffers(int a) { /* NO-OP */ }

    public static void glDeleteFramebuffers(int a) { /* NO-OP */ }

    public static int glGenFramebuffers() { /* NO-OP */ return 0; }

    public static int glGenRenderbuffers() { /* NO-OP */ return 0; }

    public static void glRenderbufferStorage(int a, int b, int c, int d) { /* NO-OP */ }

    public static void glFramebufferRenderbuffer(int a, int b, int c, int d) { /* NO-OP */ }

    public static int glCheckFramebufferStatus(int a) { /* NO-OP */ return 0; }

    public static void glFramebufferTexture2D(int a, int b, int c, int d, int e) { /* NO-OP */ }

    public native static int glGenTextures();

    public native static void glBindTexture(int target, int texture);

    public native static void glTexImage2D(int target, int level, int internalFormat, int width, int height, int border, int format, int type, ByteBuffer data);

    public native static void glDeleteTextures(int texture);

    public native static void glTexParameterf(int texture, int param, float value);
    public native static void glTexParameteri(int texture, int param, int value);

    public native static float glGetTexParameterf(int texture, int param);
    public native static int glGetTexParameteri(int texture, int param);

    public native static void glBegin(int mode);
    public native static void glEnd();
}
