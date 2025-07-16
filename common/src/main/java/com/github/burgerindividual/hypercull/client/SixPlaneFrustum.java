package com.github.burgerindividual.hypercull.client;

import org.joml.Vector4f;

public interface SixPlaneFrustum {
    /**
     * @return An array of 6 planes representing the frustum
     */
    Vector4f[] hypercull$getPlanes();
}
