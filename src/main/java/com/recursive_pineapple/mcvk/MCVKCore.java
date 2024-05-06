package com.recursive_pineapple.mcvk;

import java.util.List;
import java.util.Map;
import java.util.Set;

import org.apache.logging.log4j.Logger;
import org.apache.logging.log4j.LogManager;

import com.gtnewhorizon.gtnhmixins.IEarlyMixinLoader;
import com.recursive_pineapple.mcvk.mixins.Mixins;

import cpw.mods.fml.relauncher.IFMLLoadingPlugin;

@IFMLLoadingPlugin.MCVersion("1.7.10")
@IFMLLoadingPlugin.TransformerExclusions({ "com.recursive_pineapple.mcvk" })
@IFMLLoadingPlugin.DependsOn("cofh.asm.LoadingPlugin")
public class MCVKCore implements IFMLLoadingPlugin, IEarlyMixinLoader {

    public static final Logger LOG = LogManager.getLogger("MCVKCore");
    
    @Override
    public String getMixinConfig() {
        return "mixins.mcvk.early.json";
    }

    @Override
    public List<String> getMixins(Set<String> loadedCoreMods) {
        return Mixins.getEarlyMixins(loadedCoreMods);
    }

    @Override
    public String[] getASMTransformerClass() {
        return new String[] {
            // CoreTransformer.class.getName()
        };
    }

    @Override
    public String getModContainerClass() {
        return null;
    }

    @Override
    public String getSetupClass() {
        return null;
    }

    @Override
    public void injectData(Map<String, Object> data) {}

    @Override
    public String getAccessTransformerClass() {
        return null;
    }
}
