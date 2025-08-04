package com.github.burgerindividual.hypercull.client;

import com.github.burgerindividual.hypercull.client.ffi.HyperCullNativeLib;
import it.unimi.dsi.fastutil.objects.ObjectOpenHashSet;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.util.Set;

public class HyperCullClientMod {
    public static final String MOD_ID = "hypercull";
    public static final String MOD_NAME = "HyperCull";
    public static final Logger LOGGER = LoggerFactory.getLogger(MOD_NAME);

    public static int FRAME_INVOCATIONS = 0;
    public static int FRAME_FALLBACKS = 0;

    private static final Set<String> UNSUPPORTED_FRUSTUM_CLASSES = new ObjectOpenHashSet<>();

    public static void init() {
        HyperCullNativeLib.init();
    }

    public static void logUnsupportedFrustum(Class<?> frustumClass) {
        var className = frustumClass.getName();
        var classNotLogged = UNSUPPORTED_FRUSTUM_CLASSES.add(className);

        if (classNotLogged) {
            LOGGER.warn("Unsupported frustum found with class name {}", className);
            LOGGER.warn("HyperCull will be partially or completely disabled!");
        }

        FRAME_FALLBACKS++;
    }
}
