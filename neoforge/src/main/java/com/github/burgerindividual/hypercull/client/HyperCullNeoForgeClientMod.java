package com.github.burgerindividual.hypercull.client;


import net.neoforged.api.distmarker.Dist;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.fml.common.Mod;

@Mod(value = HyperCullClientMod.MOD_ID, dist = Dist.CLIENT)
public class HyperCullNeoForgeClientMod {
    public HyperCullNeoForgeClientMod(IEventBus eventBus) {
        HyperCullClientMod.init();
    }
}