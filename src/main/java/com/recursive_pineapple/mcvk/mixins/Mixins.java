package com.recursive_pineapple.mcvk.mixins;

import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import java.util.function.Supplier;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

import com.gtnewhorizons.neid.mixins.TargetedMod;

import cpw.mods.fml.relauncher.FMLLaunchHandler;

public enum Mixins {

    CORE(new Builder()
        .addMixin("net.minecraft.client.MinecraftMixins")
        .addMixin("net.minecraft.crash.CrashReportCategoryMixins")
        .addMixin("net.minecraft.client.renderer.texture.TextureMapMixins")
        .addMixin("net.minecraft.client.renderer.texture.TextureAtlasSpriteMixins")
        .addMixin("net.minecraft.client.renderer.OpenGLHelperMixins")
        .setPhase(Phase.EARLY)
        .setSide(Side.CLIENT)
    )

    ;

    private static final Logger LOG = LogManager.getLogger("MCVK");

    private final List<String> mixinClasses;
    private final List<TargetedMod> targetedMods;
    private final List<TargetedMod> conflictingMods;
    private final Supplier<Boolean> condition;
    private final Phase phase;
    private final Side side;

    public boolean isApplicable(Set<String> loadedCoreMods, Set<String> loadedMods) {
        if (this.side == Side.CLIENT && !FMLLaunchHandler.side()
            .isClient()) {
            return false;
        }

        if (this.side == Side.SERVER && !FMLLaunchHandler.side()
            .isServer()) {
            return false;
        }

        if (this.condition != null && !this.condition.get()) {
            return false;
        }

        for (TargetedMod required : this.targetedMods) {
            if (required == TargetedMod.VANILLA) {
                continue;
            }

            if (!loadedCoreMods.contains(required.modId) && !loadedMods.contains(required.modId)) {
                LOG
                    .debug("Not loading mixin {} because required mod {} is not loaded", this.name(), required.modName);
                return false;
            }
        }

        for (TargetedMod conflicting : this.conflictingMods) {
            if (conflicting == TargetedMod.VANILLA) {
                LOG
                    .warn("Mixin {} supposedly conflicts with vanilla: there is likely a typo somewhere", this.name());
                continue;
            }

            if (loadedCoreMods.contains(conflicting.modId) || loadedMods.contains(conflicting.modId)) {
                LOG.debug(
                    "Not loading mixin {} because conflicting mod {} is loaded",
                    this.name(),
                    conflicting.modName);
                return false;
            }
        }

        return true;
    }

    public static List<String> getEarlyMixins(Set<String> loadedCoreMods) {
        return getMixins(Phase.EARLY, loadedCoreMods, null);
    }

    public static List<String> getLateMixins(Set<String> loadedMods) {
        return getMixins(Phase.LATE, null, loadedMods);
    }

    private static List<String> getMixins(Phase phase, Set<String> loadedCoreMods, Set<String> loadedMods) {
        List<String> mixinClasses = new ArrayList<>();

        for (Mixins mixin : Mixins.values()) {
            if (mixin.phase == phase && mixin.isApplicable(loadedCoreMods, loadedMods)) {
                mixinClasses.addAll(mixin.mixinClasses);
            }
        }

        LOG.info("Loading the following mixins: {}", mixinClasses);

        return mixinClasses;
    }

    private Mixins(Builder builder) {
        this.mixinClasses = builder.mixinClasses;
        this.targetedMods = builder.targetedMods == null ? new ArrayList<>() : builder.targetedMods;
        this.conflictingMods = builder.conflictingMods;
        this.condition = builder.condition;
        this.phase = builder.phase;
        this.side = builder.side == null ? Side.CLIENT : builder.side;

        if (this.mixinClasses.isEmpty()) {
            throw new RuntimeException("No mixin classes specified for mixin: " + this.name());
        }

        if (this.phase == null) {
            throw new RuntimeException("No Phase specified for mixin: " + this.name());
        }
    }

    @SuppressWarnings("unused")
    private static class Builder {

        final List<String> mixinClasses = new ArrayList<>();
        final List<TargetedMod> targetedMods = new ArrayList<>();
        final List<TargetedMod> conflictingMods = new ArrayList<>();
        Supplier<Boolean> condition = null;
        Phase phase = null;
        Side side = null;

        public Builder addMixin(String clazz) {
            mixinClasses.add(clazz);
            return this;
        }

        public Builder addTargetMod(TargetedMod mod) {
            targetedMods.add(mod);
            return this;
        }

        public Builder addConflictingMod(TargetedMod mod) {
            conflictingMods.add(mod);
            return this;
        }

        public Builder setCondition(Supplier<Boolean> condition) {
            this.condition = condition;
            return this;
        }

        public Builder setPhase(Phase phase) {
            this.phase = phase;
            return this;
        }

        public Builder setSide(Side side) {
            this.side = side;
            return this;
        }
    }

    private enum Side {
        BOTH,
        CLIENT,
        SERVER
    }

    private enum Phase {
        EARLY,
        LATE,
    }
}
