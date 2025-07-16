package com.github.burgerindividual.hypercull.mixin;

import com.github.burgerindividual.hypercull.client.FrustumIntersectionAccessor;
import com.github.burgerindividual.hypercull.client.SixPlaneFrustum;
import net.caffeinemc.mods.sodium.client.render.viewport.frustum.SimpleFrustum;
import org.joml.FrustumIntersection;
import org.joml.Vector4f;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;

@Mixin(value = SimpleFrustum.class, remap = false)
public class MixinFrustum implements SixPlaneFrustum {
    @Shadow
    @Final
    private FrustumIntersection frustum;

    @Override
    public Vector4f[] hypercull$getPlanes() {
        return FrustumIntersectionAccessor.getPlanes(this.frustum);
    }
}
