package com.recursive_pineapple.mcvk.rendering;

import java.nio.ByteBuffer;
import java.nio.IntBuffer;

public class ImageData {
    public String name;
    public boolean is_image;
    
    public ByteBuffer bytes;

    public int width, height;
    public IntBuffer image;
}