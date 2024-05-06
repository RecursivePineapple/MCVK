package com.recursive_pineapple.mcvk;

import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import com.gtnewhorizons.retrofuturabootstrap.api.RfbClassTransformer;
import com.gtnewhorizons.retrofuturabootstrap.api.RfbPlugin;
import com.recursive_pineapple.mcvk.asm.CoreTransformer;

public class MCVKRfbPlugin implements RfbPlugin {

    public MCVKRfbPlugin() {

    }
    
    @Override
    public @NotNull RfbClassTransformer @Nullable [] makeEarlyTransformers() {
        return new RfbClassTransformer[] {
            new CoreTransformer()
        };
    }
}
