package com.github.burgerindividual.hypercull.mixin;

import net.caffeinemc.mods.sodium.client.render.viewport.Viewport;
import net.caffeinemc.mods.sodium.client.render.viewport.frustum.Frustum;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

@Mixin(value = Viewport.class, remap = false)
public interface ViewportAccessor {
    @Accessor
    Frustum getFrustum();
}
