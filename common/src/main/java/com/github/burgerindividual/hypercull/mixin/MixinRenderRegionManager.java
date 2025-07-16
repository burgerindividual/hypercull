package com.github.burgerindividual.hypercull.mixin;

import com.github.burgerindividual.hypercull.client.RegionAccess;
import it.unimi.dsi.fastutil.longs.Long2ReferenceOpenHashMap;
import net.caffeinemc.mods.sodium.client.render.chunk.region.RenderRegion;
import net.caffeinemc.mods.sodium.client.render.chunk.region.RenderRegionManager;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;

@Mixin(value = RenderRegionManager.class, remap = false)
public class MixinRenderRegionManager implements RegionAccess {
    @Shadow
    @Final
    private Long2ReferenceOpenHashMap<RenderRegion> regions;

    @Override
    public RenderRegion hypercull$get(int x, int y, int z) {
        var key = RenderRegion.key(x, y, z);
        return this.regions.get(key);
    }
}
