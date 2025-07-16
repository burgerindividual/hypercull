package com.github.burgerindividual.hypercull.client;

import net.fabricmc.api.ClientModInitializer;

public class HyperCullFabricClientMod implements ClientModInitializer {
    @Override
    public void onInitializeClient() {
        HyperCullClientMod.init();
    }
}
