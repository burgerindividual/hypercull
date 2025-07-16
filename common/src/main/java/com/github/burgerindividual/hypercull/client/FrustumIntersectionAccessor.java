package com.github.burgerindividual.hypercull.client;

import org.joml.FrustumIntersection;
import org.joml.Vector4f;

import java.lang.invoke.MethodHandle;
import java.lang.invoke.MethodHandles;

public class FrustumIntersectionAccessor {
    private static final MethodHandle FRUSTUM_PLANES_HANDLE = getFrustumPlanesHandle();

    private static MethodHandle getFrustumPlanesHandle() {
        try {
            var field = FrustumIntersection.class.getDeclaredField("planes");
            field.setAccessible(true);
            return MethodHandles.lookup().unreflectGetter(field);
        } catch (Exception e) {
            throw new RuntimeException("Unable to get frustum planes handle", e);
        }
    }

    public static Vector4f[] getPlanes(FrustumIntersection frustum) {
        // should be faster than normal reflection
        try {
            return (Vector4f[]) FRUSTUM_PLANES_HANDLE.invokeExact(frustum);
        } catch (Throwable t) {
            throw new RuntimeException("Failed to extract planes from frustum", t);
        }
    }
}
