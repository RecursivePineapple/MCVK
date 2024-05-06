package com.recursive_pineapple.mcvk.asm;

import java.util.Arrays;
import java.util.List;
import java.util.function.Consumer;
import java.util.function.Supplier;
import java.util.jar.Manifest;

import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.ClassWriter;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.tree.AbstractInsnNode;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.FieldInsnNode;
import org.objectweb.asm.tree.InsnList;
import org.objectweb.asm.tree.LdcInsnNode;
import org.objectweb.asm.tree.MethodInsnNode;
import org.objectweb.asm.tree.TypeInsnNode;

import com.gtnewhorizons.retrofuturabootstrap.api.ClassNodeHandle;
import com.gtnewhorizons.retrofuturabootstrap.api.ExtensibleClassLoader;
import com.gtnewhorizons.retrofuturabootstrap.api.RfbClassTransformer;
import com.recursive_pineapple.mcvk.MCVKCore;

import net.minecraft.launchwrapper.IClassTransformer;

public class CoreTransformer implements RfbClassTransformer {

    private static final String GL = "org/lwjgl/opengl/GL";

    private static final ClassConstantPoolParser ccpp = new ClassConstantPoolParser(
        GL
    );

    private static final List<String> classBlacklist = Arrays.asList(

    );

    private static final List<String> packageBlacklist = Arrays.asList(
        "org/lwjglx",
        "com/recursive_pineapple/mcvk"
    );

    @Override
    public @NotNull String id() {
        return "mcvk-core";
    }

    @Override
    public boolean shouldTransformClass(
        @NotNull ExtensibleClassLoader classLoader,
        @NotNull Context context,
        @Nullable Manifest manifest,
        @NotNull String className,
        @NotNull ClassNodeHandle classNode
    ) {
        if(true) throw new RuntimeException("AAAAAAAAAAAAAAAAAAAAAAAAAA");

        if(className.equals("org.lwjglx.opengl.Display")) {
            return true;
        }

        if(className.equals("cpw.mods.fml.client.SplashProgress")) {
            return true;
        }

        if(ccpp.find(classNode.getOriginalBytes())) {
            return true;
        }

        return false;
    }

    @Override
    public void transformClass(
        @NotNull ExtensibleClassLoader classLoader,
        @NotNull Context context,
        @Nullable Manifest manifest,
        @NotNull String className,
        @NotNull ClassNodeHandle classNode
    ) {
        if(className.equals("org.lwjglx.opengl.Display")) {
            transformDisplay(classNode.getNode());
        }

        if(className.equals("cpw.mods.fml.client.SplashProgress")) {
            transformSplashProgress(classNode.getNode());
        }

        if(ccpp.find(classNode.getOriginalBytes())) {

        }
    }

    private void tryRedirectOpenGL(String name, ClassNodeHandle classNode) {

        if(classBlacklist.contains(name)) {
            return;
        }

        for(var pkg : packageBlacklist) {
            if(name.startsWith(pkg)) {
                return;
            }
        }

        if(!ccpp.find(classNode.getOriginalBytes(), true)) {
            return;
        }

        redirectOpenGL(classNode.getNode());
    }

    private byte[] transform(byte[] basicClass, Consumer<ClassNode> transformer) {
        ClassReader reader = new ClassReader(basicClass);

        ClassNode node = new ClassNode();
        reader.accept(node, 0);

        transformer.accept(node);

        ClassWriter writer = new ClassWriter(reader, 0);
        node.accept(writer);

        return writer.toByteArray();
    }

    private void transformDisplay(ClassNode display) {
        for(var method: display.methods) {
            if(method.name.equals("create")) {
                injectInsns(
                    method.instructions,
                    new InsnPredicate[] {
                        isGetStatic("org/lwjglx/opengl/Display$Window", null, "handle"),
                        isInvokeStatic("org/lwjgl/glfw/GLFW", null, "glfwMakeContextCurrent")
                    },
                    () -> new AbstractInsnNode[] {
                        new FieldInsnNode(Opcodes.GETSTATIC, "org/lwjglx/opengl/Display$Window", "handle", "J"),
                        new MethodInsnNode(Opcodes.INVOKESTATIC, "com/recursive_pineapple/mcvk/rendering/VkInstance", "init", "(J)V")
                    }
                );

                injectInsns(
                    method.instructions,
                    new InsnPredicate[] {
                        isInvokeStatic("org/lwjgl/glfw/GLFW", null, "glfwDefaultWindowHints")
                    },
                    () -> new AbstractInsnNode[] {
                        new LdcInsnNode(139265 /* GLFW_CLIENT_API */),
                        new LdcInsnNode(0),
                        new MethodInsnNode(Opcodes.INVOKESTATIC, "org/lwjgl/glfw/GLFW", "glfwWindowHint", "(II)V")
                    }
                );

                removeInsns(
                    method.instructions,
                    new InsnPredicate[] {
                        isGetStatic("org/lwjglx/opengl/Display$Window", null, "handle"),
                        isInvokeStatic("org/lwjgl/glfw/GLFW", null, "glfwMakeContextCurrent")
                    }
                );

                removeInsns(
                    method.instructions,
                    new InsnPredicate[] {
                        isNew("org/lwjglx/opengl/DrawableGL"),
                        isBasic(Opcodes.DUP),
                        isInit("org/lwjglx/opengl/DrawableGL", null),
                        isPutStatic(null, "Lorg/lwjglx/opengl/DrawableGL;", "drawable")
                    }
                );

                removeInsns(
                    method.instructions,
                    new InsnPredicate[] {
                        isNew("org/lwjglx/opengl/DrawableGL"),
                        isBasic(Opcodes.DUP),
                        isInit("org/lwjglx/opengl/DrawableGL", null),
                        isPutStatic(null, "Lorg/lwjglx/opengl/DrawableGL;", "drawable")
                    }
                );

                removeInsns(
                    method.instructions,
                    new InsnPredicate[] {
                        isInvokeStatic("org/lwjgl/opengl/GL", null, "createCapabilities"),
                        isBasic(Opcodes.POP),
                    }
                );

            }
        }
    }

    private void transformSplashProgress(ClassNode splashProgress) {
        for(var method : splashProgress.methods) {
            if(method.name.equals("start")) {
                removeInsns(method.instructions, new InsnPredicate[] {
                    isNew("org/lwjgl/opengl/SharedDrawable"),
                    isBasic(Opcodes.DUP),
                    isInvokeStatic(null, null, "getDrawable"),
                    isInit("org/lwjgl/opengl/SharedDrawable", null),
                    isPutStatic(null, null, "d"),
                    isInvokeStatic("org/lwjgl/opengl/Display", null, "getDrawable"),
                    isInvokeInterface("org/lwjgl/opengl/Drawable", null, "releaseContext"),
                    isGetStatic(null, "Lorg/lwjgl/opengl/Drawable", "d"),
                    isInvokeInterface("org/lwjgl/opengl/Drawable", null, "makeCurrent")
                });
            }
        }
    }

    private void redirectOpenGL(ClassNode target) {
        for(var method : target.methods) {
            var insn = method.instructions.getFirst();
            while(insn != null) {
                if(insn instanceof MethodInsnNode methodInsn) {
                    if(methodInsn.owner.startsWith(GL)) {
                        MCVKCore.LOG.trace(
                            "Redirecting OpenGL call {}.{}{} in method {}.{}{}",
                            methodInsn.owner, methodInsn.name, methodInsn.desc,
                            target.name, method.name, method.desc
                        );
                        methodInsn.owner = "com/recursive_pineapple/mcvk/rendering/RenderSandbox";
                    }
                }

                insn = insn.getNext();
            }
        }
    }

    @FunctionalInterface
    static interface InsnPredicate {
        public boolean test(AbstractInsnNode node);
    }

    static boolean injectInsns(InsnList list, InsnPredicate[] matchers, Supplier<AbstractInsnNode[]> toInject) {
        if(matchers.length == 0) {
            return false;
        }

        AbstractInsnNode current = list.getFirst();

        boolean injectedSomething = false;

        while(current != null) {
            int i = 0;

            AbstractInsnNode needle = current;

            while(needle != null && i < matchers.length && matchers[i].test(needle)) {
                i++;
                needle = current.getNext();
            }

            if(i == matchers.length) {
                AbstractInsnNode[] inject = toInject.get();

                for(var node : inject) {
                    list.insert(current, node);
                    current = current.getNext();
                }

                injectedSomething = true;
            }

            current = current.getNext();
        }

        return injectedSomething;
    }

    static boolean removeInsns(InsnList list, InsnPredicate[] matchers) {
        if(matchers.length == 0) {
            return false;
        }

        AbstractInsnNode current = list.getFirst();

        AbstractInsnNode[] toRemove = new AbstractInsnNode[matchers.length];

        boolean removedSomething = false;

        while(current != null) {
            int i = 0;

            AbstractInsnNode needle = current;

            while(needle != null && i < matchers.length && matchers[i].test(needle)) {
                toRemove[i] = needle;
                i++;
                needle = current.getNext();
            }

            if(i == matchers.length) {
                current = toRemove[toRemove.length - 1].getNext();

                for(AbstractInsnNode node : toRemove) {
                    list.remove(node);
                }

                removedSomething = true;
            } else {
                current = current.getNext();
            }
        }

        return removedSomething;
    }

    static InsnPredicate isGetStatic(@Nullable String owner, @Nullable String desc, @Nullable String name) {
        return insn -> insn.getOpcode() == Opcodes.GETSTATIC &&
            insn instanceof FieldInsnNode node &&
            (owner == null || owner.equals(node.owner)) && 
            (desc == null || desc.equals(node.desc)) && 
            (name == null || name.equals(node.name));
    }

    static InsnPredicate isPutStatic(@Nullable String owner, @Nullable String desc, @Nullable String name) {
        return insn -> insn.getOpcode() == Opcodes.PUTSTATIC &&
            insn instanceof FieldInsnNode node &&
            (owner == null || owner.equals(node.owner)) && 
            (desc == null || desc.equals(node.desc)) && 
            (name == null || name.equals(node.name));
    }

    static InsnPredicate isInvokeStatic(@Nullable String owner, @Nullable String desc, @Nullable String name) {
        return insn -> insn.getOpcode() == Opcodes.INVOKESTATIC &&
            insn instanceof MethodInsnNode node &&
            (owner == null || owner.equals(node.owner)) && 
            (desc == null || desc.equals(node.desc)) && 
            (name == null || name.equals(node.name)) && 
            node.itf == false;
    }

    static InsnPredicate isInvokeInterface(@Nullable String owner, @Nullable String desc, @Nullable String name) {
        return insn -> insn.getOpcode() == Opcodes.INVOKEINTERFACE &&
            insn instanceof MethodInsnNode node &&
            (owner == null || owner.equals(node.owner)) && 
            (desc == null || desc.equals(node.desc)) && 
            (name == null || name.equals(node.name)) && 
            node.itf == true;
    }

    static InsnPredicate isNew(@Nullable String desc) {
        return insn -> insn.getOpcode() == Opcodes.NEW &&
            insn instanceof TypeInsnNode node &&
            (desc == null || desc.equals(node.desc));
    }

    static InsnPredicate isInit(@Nullable String owner, @Nullable String desc) {
        return insn -> insn.getOpcode() == Opcodes.INVOKESPECIAL &&
            insn instanceof MethodInsnNode node &&
            (owner == null || owner.equals(node.owner)) && 
            node.name.equals("<init>") && 
            (desc == null || desc.equals(node.desc)) &&
            !node.itf;
    }

    static InsnPredicate isBasic(int opcode) {
        return insn -> insn.getOpcode() == opcode;
    }
}
