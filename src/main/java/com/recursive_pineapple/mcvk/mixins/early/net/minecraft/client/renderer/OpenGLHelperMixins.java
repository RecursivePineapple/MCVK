package com.recursive_pineapple.mcvk.mixins.early.net.minecraft.client.renderer;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

import net.minecraft.client.renderer.OpenGlHelper;

@Mixin(OpenGlHelper.class)
public class OpenGLHelperMixins {
    
    @Inject(method = "initializeTextures", at = @At("HEAD"), cancellable = true)
    private static void cancelInitializeTextures(CallbackInfo ci) {
        ci.cancel();
    }

}
