package com.github.burgerindividual.hypercull.mixin.sodium;

import com.github.burgerindividual.hypercull.client.FrustumIntersectionAccessor;
import com.github.burgerindividual.hypercull.client.FrustumPlaneProvider;
import net.caffeinemc.mods.sodium.client.render.viewport.frustum.SimpleFrustum;
import org.jetbrains.annotations.NotNull;
import org.joml.FrustumIntersection;
import org.joml.Vector4f;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;

@Mixin(value = SimpleFrustum.class, remap = false)
public class MixinSimpleFrustum implements FrustumPlaneProvider {
    @Shadow
    @Final
    private FrustumIntersection frustum;

    @Override
    public @NotNull Vector4f[] hypercull$getPlanes() {
        return FrustumIntersectionAccessor.getPlanes(this.frustum);
    }
}
