package com.github.burgerindividual.hypercull.client;

import net.caffeinemc.mods.sodium.client.render.chunk.region.RenderRegion;

public interface RegionAccess {
    RenderRegion hypercull$get(int x, int y, int z);
}
