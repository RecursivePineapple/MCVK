package com.recursive_pineapple.mcvk.mixins.early.net.minecraft.client;

import java.util.concurrent.Callable;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Overwrite;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.Redirect;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.points.AfterInvoke;

import com.recursive_pineapple.mcvk.MCVK;
import com.recursive_pineapple.mcvk.rendering.VkInstance;

import net.minecraft.client.Minecraft;
import net.minecraft.crash.CrashReportCategory;

@Mixin(Minecraft.class)
public class MinecraftMixins {
    
    @Inject(method = "loadScreen", at = @At("HEAD"), cancellable = true)
    private void stopLoadingScreen(CallbackInfo ci) {
        ci.cancel();
    }

    @Inject(method = "runGameLoop", at = @At("HEAD"))
    private void startFrame(CallbackInfo _ci) {
        VkInstance.getInstance().startFrame();
    }

    @Inject(method = "runGameLoop", at = @At("TAIL"))
    private void finishFrame(CallbackInfo _ci) {
        VkInstance.getInstance().finishFrame();
    }
}
