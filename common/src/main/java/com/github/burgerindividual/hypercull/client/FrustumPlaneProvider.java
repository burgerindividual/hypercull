package com.github.burgerindividual.hypercull.client;

import org.jetbrains.annotations.NotNull;
import org.joml.Vector4f;

public interface FrustumPlaneProvider {
    /**
     * @return An array of 16 or fewer planes representing the frustum
     */
    @NotNull Vector4f[] hypercull$getPlanes();
}
