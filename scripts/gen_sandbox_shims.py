#!/usr/bin/env python3

import os, sys

members = []

# generate SetPointer proxies
buffer_types = [
    ("ByteBuffer", "ITEM_TYPE_BYTES"),
    ("ShortBuffer", "ITEM_TYPE_SHORTS"),
    ("IntBuffer", "ITEM_TYPE_INTS"),
    ("FloatBuffer", "ITEM_TYPE_FLOATS"),
    ("DoubleBuffer", "ITEM_TYPE_DOUBLES"),
]

array_types = [
    ("ARRAY_TYPE_COLOR", "Color"),
    ("ARRAY_TYPE_COLOR_SECONDARY", "SecondaryColor"),
    ("ARRAY_TYPE_INDEX", "Index"),
    ("ARRAY_TYPE_NORMAL", "Normal"),
    ("ARRAY_TYPE_TEXCOORD", "TexCoord"),
    ("ARRAY_TYPE_VERTEX", "Vertex"),
]

for (buffer_class, buffer_type) in buffer_types:
    for (array_type, method_name) in array_types:
        members.append(f"public static void gl{method_name}Pointer(int size, int stride, {buffer_class} pointer) {{")
        members.append(f"    RenderSandbox.addPointerArray(size, stride, RenderSandbox.{array_type}, RenderSandbox.{buffer_type}, MemoryUtil.getAddress(pointer), pointer.remaining());")
        members.append(f"}}")

gl_fns = [
    ("glVertex", [2, 3, 4]),
    ("glTexCoord", [2, 3, 4]),
    ("glNormal", [2, 3, 4]),
    ("glColor", [3, 4]),
]

data_types = [
    ("f", "float"),
    ("d", "double"),
    ("i", "int"),
    ("s", "short"),
    ("b", "byte"),
    ("l", "long")
]

len_params = [
    "",
    "{t} x",
    "{t} x, {t} y",
    "{t} x, {t} y, {t} z",
    "{t} x, {t} y, {t} z, {t} w",
]

for (prefix, lens) in gl_fns:
    for l in lens:
        for (suffix, java_type) in data_types:
            params = len_params[l].format(t=java_type)
            members.append(f"public static native void {prefix}{l}{suffix}({params});")

out_path = os.path.join(
    os.path.dirname(__file__), "..",
    "src", "main", "java",
    "com", "recursive_pineapple", "mcvk",
    "rendering", "RenderSandboxGen.java"
)

with open(out_path, "w") as out:
    lines = [
        "package com.recursive_pineapple.mcvk.rendering;"
        "",
        "",
        "import org.lwjgl.MemoryUtil;",
        "import java.nio.ByteBuffer;",
        "import java.nio.DoubleBuffer;",
        "import java.nio.FloatBuffer;",
        "import java.nio.IntBuffer;",
        "import java.nio.ShortBuffer;",
        "",
        "public class RenderSandboxGen {",
        *[f"    {member}" for member in members],
        "}"
    ]
    out.writelines([line + "\n" for line in lines])
