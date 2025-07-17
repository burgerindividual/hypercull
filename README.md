# HyperCull
### An attempt at a faster culling algorithm for Minecraft, built to slot into the existing architecture of the [Sodium](https://github.com/caffeinemc/sodium/) world renderer

This repository contains an implementation of tile-based culling, which was initially theorized by JellySquid. While only occlusion culling was considered in the original theory, this has been expanded to include tile-based implementations of fog culling, angle culling, and frustum culling successfully. Combined, the number of sections culled should be on-par with the current Java implementation, while being much more efficient.

# Building
### This project is not ready to be tested! 
However, if you still would like to build the project, the following instructions are provided:

// TODO

### Native Compiled Platforms:
```
linux-x64-avx2+fma
macos-x64-avx2+fma
windows-x64-avx2+fma
linux-x64-sse4_1+ssse3
macos-x64-sse4_1+ssse3
windows-x64-sse4_1+ssse3
linux-arm64
macos-arm64
windows-arm64
```

The Java code which loads the native library will automatically choose the best option of the above list for the system it's running on.

# How it works
(TODO)