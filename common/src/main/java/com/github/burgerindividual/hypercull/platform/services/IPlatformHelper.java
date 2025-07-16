package com.github.burgerindividual.hypercull.platform.services;

import java.nio.file.Path;

public interface IPlatformHelper {

    /**
     * @return The name of the current platform.
     */
    String getPlatformName();

    /**
     * Checks if a mod with the given id is loaded.
     *
     * @param modId The mod to check if it is loaded.
     * @return True if the mod is loaded, false otherwise.
     */
    boolean isModLoaded(String modId);

    /**
     * @return True if in a development environment, false otherwise.
     */
    boolean isDevelopmentEnvironment();

    /**
     * @return The root path of the game
     */
    Path getGameDir();
}