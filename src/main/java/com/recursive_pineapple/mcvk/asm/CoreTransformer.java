package com.recursive_pineapple.mcvk.asm;

import java.io.IOException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.function.Supplier;
import java.util.jar.Manifest;

import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;
import org.objectweb.asm.ClassReader;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.tree.AbstractInsnNode;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.FieldInsnNode;
import org.objectweb.asm.tree.InsnNode;
import org.objectweb.asm.tree.LabelNode;
import org.objectweb.asm.tree.LdcInsnNode;
import org.objectweb.asm.tree.LocalVariableNode;
import org.objectweb.asm.tree.MethodInsnNode;
import org.objectweb.asm.tree.MethodNode;
import org.objectweb.asm.tree.TypeInsnNode;
import org.objectweb.asm.tree.VarInsnNode;
import org.spongepowered.asm.lib.Type;

import com.gtnewhorizons.retrofuturabootstrap.api.ClassNodeHandle;
import com.gtnewhorizons.retrofuturabootstrap.api.ExtensibleClassLoader;
import com.gtnewhorizons.retrofuturabootstrap.api.RfbClassTransformer;
import com.recursive_pineapple.mcvk.MCVKCore;

public class CoreTransformer implements RfbClassTransformer {

    private static final String GL = "org/lwjgl/opengl/GL";
    private static final String XGL = "org/lwjgl/opengl/GL";

    private static final ClassConstantPoolParser ccpp = new ClassConstantPoolParser(
        GL,
        XGL
    );

    private static final List<String> classBlacklist = Arrays.asList(

    );

    private static final List<String> packageBlacklist = Arrays.asList(
        "org.lwjglx.",
        "org.lwjgl.",
        "com.recursive_pineapple.mcvk"
    );

    private static final HashMap<String, String> renderSandboxMethods = new HashMap<>();

    static {
        var renderSandbox = loadClass("com/recursive_pineapple/mcvk/rendering/RenderSandbox.class");

        for(var method : renderSandbox.methods) {
            if(!"<init>".equals(method.name)) {
                renderSandboxMethods.put(method.name + method.desc, "com/recursive_pineapple/mcvk/rendering/RenderSandbox");
            }
        }

        var renderSandboxGen = loadClass("com/recursive_pineapple/mcvk/rendering/RenderSandboxGen.class");

        for(var method : renderSandboxGen.methods) {
            if(!"<init>".equals(method.name)) {
                renderSandboxMethods.put(method.name + method.desc, "com/recursive_pineapple/mcvk/rendering/RenderSandboxGen");
            }
        }
    }

    private static ClassNode loadClass(String path) {
        try {
            ClassReader reader = new ClassReader(CoreTransformer.class.getClassLoader().getResourceAsStream(path));

            ClassNode node = new ClassNode();
            reader.accept(node, 0);

            return node;
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }

    public CoreTransformer() {

    }


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
        if(className.equals("org.lwjglx.opengl.Display")) {
            return true;
        }

        if(className.equals("cpw.mods.fml.client.SplashProgress")) {
            return true;
        }

        if(ccpp.find(classNode.getOriginalBytes(), true)) {
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

        tryRedirectOpenGL(className, classNode);
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

    private void transformDisplay(ClassNode display) {
        for(var method: display.methods) {
            if(method.name.equals("create")) {
                injectInsns(
                    method,
                    new InsnPredicate[] {
                        isGetStatic("org/lwjglx/opengl/Display$Window", null, "handle"),
                        isInvokeStatic("org/lwjgl/glfw/GLFW", null, "glfwMakeContextCurrent")
                    },
                    () -> new AbstractInsnNode[] {
                        new FieldInsnNode(Opcodes.GETSTATIC, "org/lwjglx/opengl/Display$Window", "handle", "J"),
                        new MethodInsnNode(Opcodes.INVOKESTATIC, "com/recursive_pineapple/mcvk/rendering/MCVKNative", "init", "(J)V", false)
                    }
                );

                injectInsns(
                    method,
                    new InsnPredicate[] {
                        isInvokeStatic("org/lwjgl/glfw/GLFW", null, "glfwDefaultWindowHints")
                    },
                    () -> new AbstractInsnNode[] {
                        new LdcInsnNode(139265 /* GLFW_CLIENT_API */),
                        new LdcInsnNode(0),
                        new MethodInsnNode(Opcodes.INVOKESTATIC, "org/lwjgl/glfw/GLFW", "glfwWindowHint", "(II)V", false)
                    }
                );

                removeInsns(
                    method,
                    new InsnPredicate[] {
                        isGetStatic("org/lwjglx/opengl/Display$Window", null, "handle"),
                        isInvokeStatic("org/lwjgl/glfw/GLFW", null, "glfwMakeContextCurrent")
                    }
                );

                removeInsns(
                    method,
                    new InsnPredicate[] {
                        isNew("org/lwjglx/opengl/DrawableGL"),
                        isBasic(Opcodes.DUP),
                        isInit("org/lwjglx/opengl/DrawableGL", null),
                        isPutStatic(null, "Lorg/lwjglx/opengl/DrawableGL;", "drawable")
                    }
                );

                removeInsns(
                    method,
                    new InsnPredicate[] {
                        isNew("org/lwjglx/opengl/DrawableGL"),
                        isBasic(Opcodes.DUP),
                        isInit("org/lwjglx/opengl/DrawableGL", null),
                        isPutStatic(null, "Lorg/lwjglx/opengl/DrawableGL;", "drawable")
                    }
                );

                removeInsns(
                    method,
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
                var matchers = new InsnPredicate[] {
                    isNew("org/lwjgl/opengl/SharedDrawable"),
                    isBasic(Opcodes.DUP),
                    isInvoke(null, null, "getDrawable"),
                    isInit("org/lwjgl/opengl/SharedDrawable", null),
                    isPutStatic(null, null, "d"),
                    isInvoke(null, null, "getDrawable"),
                    isInvoke(null, null, "releaseContext"),
                    isGetStatic(null, null, "d"),
                    isInvoke("org/lwjgl/opengl/Drawable", null, "makeCurrent"),
                    isBasic(Opcodes.GOTO),
                    isStoreVar(null, "e"),
                    isGetVar(null, "e"),
                    isInvoke(null, null, "printStackTrace"),
                    isNew("java/lang/RuntimeException"),
                    isBasic(Opcodes.DUP),
                    isGetVar(null, "e"),
                    isInit("java/lang/RuntimeException", null),
                    isBasic(Opcodes.ATHROW),
                };

                boolean removed = removeInsns(method, matchers);

                if(!removed) {
                    throw new RuntimeException("failed to remove SplashProgress context creation insns.");
                }
            }
        }
    }

    private void redirectOpenGL(ClassNode target) {
        for(var method : target.methods) {
            var iter = method.instructions.iterator();

            while(iter.hasNext()) {
                if(iter.next() instanceof MethodInsnNode methodInsn) {
                    if(methodInsn.owner.startsWith(GL) || methodInsn.owner.startsWith(XGL) || (methodInsn.owner.equals("org.lwjglx.opengl.Display") && methodInsn.name.equals("makeCurrent"))) {
                        var sandbox = renderSandboxMethods.get(methodInsn.name + methodInsn.desc);

                        if(sandbox != null) {
                            MCVKCore.LOG.trace(
                                "Redirecting OpenGL call {}.{}{} in method {}.{}{}",
                                methodInsn.owner, methodInsn.name, methodInsn.desc,
                                target.name, method.name, method.desc
                            );

                            methodInsn.owner = sandbox;
                        } else {
                            MCVKCore.LOG.warn(
                                "Unimplemented OpenGL call {}.{}{} in method {}.{}{}: it will be removed",
                                methodInsn.owner, methodInsn.name, methodInsn.desc,
                                target.name, method.name, method.desc
                            );

                            var params = Type.getArgumentTypes(methodInsn.desc);
                            var ret = Type.getReturnType(methodInsn.desc);

                            iter.remove();

                            for(var param : params) {
                                if(param == Type.DOUBLE_TYPE || param == Type.LONG_TYPE) {
                                    iter.add(new InsnNode(Opcodes.POP2));
                                } else {
                                    iter.add(new InsnNode(Opcodes.POP));
                                }
                            }

                            if(
                                ret == Type.BOOLEAN_TYPE ||
                                ret == Type.CHAR_TYPE ||
                                ret == Type.BYTE_TYPE ||
                                ret == Type.SHORT_TYPE ||
                                ret == Type.INT_TYPE
                            ) {
                                iter.add(new InsnNode(Opcodes.ICONST_0));
                            }

                            if(
                                ret == Type.LONG_TYPE
                            ) {
                                iter.add(new InsnNode(Opcodes.LCONST_0));
                            }

                            if(
                                ret == Type.FLOAT_TYPE
                            ) {
                                iter.add(new InsnNode(Opcodes.FCONST_0));
                            }

                            if(
                                ret == Type.DOUBLE_TYPE
                            ) {
                                iter.add(new InsnNode(Opcodes.DCONST_0));
                            }

                            if(
                                ret.getSort() == Type.OBJECT
                            ) {
                                iter.add(new InsnNode(Opcodes.ACONST_NULL));
                            }
                        }
                    }
                }
            }
        }
    }

    @FunctionalInterface
    static interface InsnPredicate {
        public boolean test(MethodNode method, AbstractInsnNode node);
    }

    static interface InsnConsumer {
        public void consume(AbstractInsnNode first, AbstractInsnNode last, List<AbstractInsnNode> contents);
    }

    static boolean findInsns(MethodNode method, InsnPredicate[] matchers, InsnConsumer consumer) {
        if(matchers.length == 0) {
            return false;
        }

        AbstractInsnNode current = method.instructions.getFirst();

        ArrayList<AbstractInsnNode> contents = new ArrayList<>();

        boolean foundAnything = false;

        while(current != null) {
            int i = 0;

            AbstractInsnNode cursor = current;

            contents.clear();

            while(cursor != null && i < matchers.length) {
                if(cursor.getOpcode() != -1) {
                    if(matchers[i].test(method, cursor)) {
                        i++;
                    } else {
                        break;
                    }
                }

                contents.add(cursor);
                cursor = cursor.getNext();
            }

            if(i == matchers.length) {
                AbstractInsnNode first = current;
                current = cursor.getNext();

                consumer.consume(first, cursor, contents);

                foundAnything = true;
            } else {
                current = current.getNext();
            }
        }

        return foundAnything;
    }

    static boolean injectInsns(MethodNode method, InsnPredicate[] matchers, Supplier<AbstractInsnNode[]> toInject) {
        return findInsns(method, matchers, (first, last, nodes) -> {
            AbstractInsnNode[] inject = toInject.get();

            for(var node : inject) {
                method.instructions.insert(last, node);
                last = node;
            }
        });
    }

    static boolean removeInsns(MethodNode method, InsnPredicate[] matchers) {
        return findInsns(method, matchers, (first, last, nodes) -> {
            for(var node : nodes) {
                method.instructions.remove(node);

                if(node instanceof LabelNode label) {
                    var iter = method.tryCatchBlocks.iterator();

                    while(iter.hasNext()) {
                        if(iter.next().start == label) {
                            iter.remove();
                        }
                    }
                }
            }
        });
    }

    static InsnPredicate isGetStatic(@Nullable String owner, @Nullable String desc, @Nullable String name) {
        return (method, insn) -> insn.getOpcode() == Opcodes.GETSTATIC &&
            insn instanceof FieldInsnNode node &&
            (owner == null || owner.equals(node.owner)) && 
            (desc == null || desc.equals(node.desc)) && 
            (name == null || name.equals(node.name));
    }

    static InsnPredicate isPutStatic(@Nullable String owner, @Nullable String desc, @Nullable String name) {
        return (method, insn) -> insn.getOpcode() == Opcodes.PUTSTATIC &&
            insn instanceof FieldInsnNode node &&
            (owner == null || owner.equals(node.owner)) && 
            (desc == null || desc.equals(node.desc)) && 
            (name == null || name.equals(node.name));
    }

    static InsnPredicate isGetVar(@Nullable String desc, @Nullable String name) {
        return (method, insn) -> {
            if(insn.getOpcode() == Opcodes.ALOAD && insn instanceof VarInsnNode node) {
                LocalVariableNode var = ASMUtils.getVariableNode(method.localVariables, node.var);

                return var != null && (desc == null || desc.equals(var.desc)) && (name == null || name.equals(var.name));
            } else {
                return false;
            }
        };  
    }

    static InsnPredicate isStoreVar(@Nullable String desc, @Nullable String name) {
        return (method, insn) -> {
            if(insn.getOpcode() == Opcodes.ASTORE && insn instanceof VarInsnNode node) {
                LocalVariableNode var = ASMUtils.getVariableNode(method.localVariables, node.var);

                return var != null && (desc == null || desc.equals(var.desc)) && (name == null || name.equals(var.name));
            } else {
                return false;
            }
        };  
    }

    static InsnPredicate isInvokeStatic(@Nullable String owner, @Nullable String desc, @Nullable String name) {
        return (method, insn) -> insn.getOpcode() == Opcodes.INVOKESTATIC &&
            insn instanceof MethodInsnNode node &&
            (owner == null || owner.equals(node.owner)) && 
            (desc == null || desc.equals(node.desc)) && 
            (name == null || name.equals(node.name)) && 
            node.itf == false;
    }

    static InsnPredicate isInvokeInterface(@Nullable String owner, @Nullable String desc, @Nullable String name) {
        return (method, insn) -> insn.getOpcode() == Opcodes.INVOKEINTERFACE &&
            insn instanceof MethodInsnNode node &&
            (owner == null || owner.equals(node.owner)) && 
            (desc == null || desc.equals(node.desc)) && 
            (name == null || name.equals(node.name)) && 
            node.itf == true;
    }

    static InsnPredicate isInvoke(@Nullable String owner, @Nullable String desc, @Nullable String name) {
        return (method, insn) -> insn instanceof MethodInsnNode node &&
            (owner == null || owner.equals(node.owner)) && 
            (desc == null || desc.equals(node.desc)) && 
            (name == null || name.equals(node.name));
    }

    static InsnPredicate isNew(@Nullable String desc) {
        return (method, insn) -> insn.getOpcode() == Opcodes.NEW &&
            insn instanceof TypeInsnNode node &&
            (desc == null || desc.equals(node.desc));
    }

    static InsnPredicate isInit(@Nullable String owner, @Nullable String desc) {
        return (method, insn) -> insn.getOpcode() == Opcodes.INVOKESPECIAL &&
            insn instanceof MethodInsnNode node &&
            (owner == null || owner.equals(node.owner)) && 
            node.name.equals("<init>") && 
            (desc == null || desc.equals(node.desc)) &&
            !node.itf;
    }

    static InsnPredicate isBasic(int opcode) {
        return (method, insn) -> insn.getOpcode() == opcode;
    }
}
