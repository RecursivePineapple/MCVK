package com.recursive_pineapple.mcvk.mixins.early.net.minecraft.crash;

import java.util.concurrent.Callable;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

import net.minecraft.crash.CrashReportCategory;

@Mixin(CrashReportCategory.class)
public class CrashReportCategoryMixins {

    @Inject(method = "addCrashSectionCallable", at = @At("HEAD"), cancellable = true)
    private void removeOpenGlSections(String section, Callable<? extends Object> value, CallbackInfo ci) {
        if(section.contains("GL")) {
            ci.cancel();
        }
    }
    
}
