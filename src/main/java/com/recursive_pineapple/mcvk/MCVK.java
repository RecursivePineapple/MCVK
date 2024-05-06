package com.recursive_pineapple.mcvk;

import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

import com.recursive_pineapple.mcvk.rendering.MCVKNative;

import cpw.mods.fml.common.Mod;
import cpw.mods.fml.common.SidedProxy;
import cpw.mods.fml.common.event.FMLInitializationEvent;
import cpw.mods.fml.common.event.FMLPostInitializationEvent;
import cpw.mods.fml.common.event.FMLPreInitializationEvent;
import cpw.mods.fml.common.event.FMLServerStartingEvent;

@Mod(modid = MCVK.MODID, version = Tags.VERSION, name = "MCVK", acceptedMinecraftVersions = "[1.7.10]")
public class MCVK {

    public static final String MODID = "mcvk";
    public static final Logger LOG = LogManager.getLogger(MODID);

    private static boolean isValid = true;

    @SidedProxy(
        clientSide = "com.recursive_pineapple.mcvk.ClientProxy",
        serverSide = "com.recursive_pineapple.mcvk.CommonProxy")
    public static CommonProxy proxy;

    public static void setInvalid() {
        isValid = false;
        // todo: disable the mod
    }

    @Mod.EventHandler
    // preInit "Run before anything else. Read your config, create blocks, items, etc, and register them with the
    // GameRegistry." (Remove if not needed)
    public void preInit(FMLPreInitializationEvent event) {
        MCVKNative.load();
        
        proxy.preInit(event);
    }

    @Mod.EventHandler
    // load "Do your mod setup. Build whatever data structures you care about. Register recipes." (Remove if not needed)
    public void init(FMLInitializationEvent event) {
        proxy.init(event);
    }

    @Mod.EventHandler
    // postInit "Handle interaction with other mods, complete your setup based on this." (Remove if not needed)
    public void postInit(FMLPostInitializationEvent event) {
        proxy.postInit(event);
    }

    @Mod.EventHandler
    // register server commands in this event handler (Remove if not needed)
    public void serverStarting(FMLServerStartingEvent event) {
        proxy.serverStarting(event);
    }
}
