package com.github.burgerindividual.hypercull.mixin.iris;

import com.github.burgerindividual.hypercull.client.FrustumPlaneProvider;
import com.github.burgerindividual.hypercull.client.SearchDistanceProvider;
import net.irisshaders.iris.shadows.frustum.BoxCuller;
import net.irisshaders.iris.shadows.frustum.advanced.AdvancedShadowCullingFrustum;
import org.jetbrains.annotations.NotNull;
import org.joml.Vector4f;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;

@Mixin(value = AdvancedShadowCullingFrustum.class, remap = false)
public class MixinAdvancedShadowCullingFrustum implements FrustumPlaneProvider, SearchDistanceProvider {
    @Shadow
    @Final
    private float[][] planes;

    @Shadow
    @Final
    protected BoxCuller boxCuller;

    @Shadow
    private int planeCount;


    @Override
    public @NotNull Vector4f[] hypercull$getPlanes() {
        var planeCount = this.planeCount;
        var planeArray = new Vector4f[planeCount];

        for (int i = 0; i < planeCount; i++) {
            var x = this.planes[i][0];
            var y = this.planes[i][1];
            var z = this.planes[i][2];
            var w = this.planes[i][3];
            planeArray[i] = new Vector4f(x, y, z, w);
        }

        return planeArray;
    }

    @Override
    public float hypercull$getSearchDistance() {
        var boxCuller = (BoxCullerAccessor) this.boxCuller;
        if (boxCuller == null) {
            return Float.POSITIVE_INFINITY;
        } else {
            return (float) boxCuller.getMaxDistance();
        }
    }
}
