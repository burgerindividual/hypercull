package com.github.burgerindividual.hypercull.client;

import com.github.burgerindividual.hypercull.client.ffi.HyperCullNativeLib;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public class HyperCullClientMod {
    public static final String MOD_ID = "hypercull";
    public static final String MOD_NAME = "HyperCull";
    public static final Logger LOGGER = LoggerFactory.getLogger(MOD_NAME);

    public static void init() {
        loadNatives();
    }

    private static void loadNatives() {
        // Checking this variable will load the HyperCullNativeLib class. On class load,
        // the static initializer will run, which will attempt to load the native library
        // and initialize it. If that process fails, this variable will be false.
        if (HyperCullNativeLib.SUPPORTED) {
            LOGGER.info("Native culling library initialized successfully");
        }
    }
}
