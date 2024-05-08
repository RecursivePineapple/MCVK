package com.recursive_pineapple.mcvk.utils;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;

public class IOUtils {
    public static byte[] readStreamToBytes(InputStream is) throws IOException {
        return readStreamToBytes(is, new ByteArrayOutputStream(8192), new byte[8192]);
    }

    public static byte[] readStreamToBytes(InputStream is, ByteArrayOutputStream staging, byte[] buffer) throws IOException {

        int len;

        while ((len = is.read(buffer)) != -1) {
            staging.write(buffer, 0, len);
        }

        return staging.toByteArray();
    }
}
