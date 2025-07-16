package com.github.burgerindividual.hypercull.client;

import com.github.burgerindividual.hypercull.client.ffi.HyperCullNativeLib;
import net.caffeinemc.mods.sodium.client.render.chunk.LocalSectionIndex;
import net.caffeinemc.mods.sodium.client.render.chunk.RenderSection;
import net.caffeinemc.mods.sodium.client.render.chunk.occlusion.OcclusionCuller;
import net.caffeinemc.mods.sodium.client.render.chunk.region.RenderRegion;
import net.caffeinemc.mods.sodium.client.render.chunk.region.RenderRegionManager;
import net.caffeinemc.mods.sodium.client.render.viewport.CameraTransform;
import org.lwjgl.system.MemoryStack;
import org.lwjgl.system.MemoryUtil;

import java.io.Closeable;

public class NativeGraph implements Closeable {
    private static final int TILE_WIDTH = 8;
    private static final int TILE_HEIGHT = 8;
    private static final int TILE_LENGTH = 8;

    private static final int TILE_WIDTH_M = TILE_WIDTH - 1;
    private static final int TILE_HEIGHT_M = TILE_HEIGHT - 1;
    private static final int TILE_LENGTH_M = TILE_LENGTH - 1;

    // YZX order
    private static final int TILE_IDX_X_OFFSET = 0;
    private static final int TILE_IDX_Y_OFFSET = 6;
    private static final int TILE_IDX_Z_OFFSET = 3;

    private final long nativePtr;
    private final RenderRegionManager regions;

    public NativeGraph(RenderRegionManager regions, byte renderDistance, byte minSectionY, byte maxSectionY) {
        this.nativePtr = HyperCullNativeLib.graphCreate(
                renderDistance,
                minSectionY,
                maxSectionY
        );
        this.regions = regions;
    }

    public void findVisible(
            OcclusionCuller.Visitor visitor,
            SixPlaneFrustum frustum,
            CameraTransform transform,
            float searchDistance,
            boolean useOcclusionCulling,
            int frame
    ) {
        try (var stack = MemoryStack.stackPush()) {
            var resultsPtr = stack.ncalloc(HyperCullNativeLib.FFISLICE_ALIGNMENT, 1, HyperCullNativeLib.FFISLICE_SIZE);
            var cameraPtr = HyperCullNativeLib.frustumCreate(
                    stack,
                    frustum,
                    transform
            );

            HyperCullNativeLib.graphSearch(
                    resultsPtr,
                    this.nativePtr,
                    cameraPtr,
                    searchDistance,
                    useOcclusionCulling
            );

            var tilesDataPtr = MemoryUtil.memGetAddress(resultsPtr + HyperCullNativeLib.FFISLICE_DATA_PTR_OFFSET);
            var tileCount = MemoryUtil.memGetAddress(resultsPtr + HyperCullNativeLib.FFISLICE_COUNT_OFFSET);

            for (var tileIdx = 0L; tileIdx < tileCount; tileIdx++) {
                this.readTile(tilesDataPtr + (tileIdx * HyperCullNativeLib.FFITILE_SIZE), visitor, frame);
            }
        }
    }

    private void readTile(long tilePtr, OcclusionCuller.Visitor visitor, int frame) {
        var tileSectionX = MemoryUtil.memGetInt(tilePtr + HyperCullNativeLib.FFITILE_ORIGIN_SECTION_X_OFFSET);
        var tileSectionY = MemoryUtil.memGetInt(tilePtr + HyperCullNativeLib.FFITILE_ORIGIN_SECTION_Y_OFFSET);
        var tileSectionZ = MemoryUtil.memGetInt(tilePtr + HyperCullNativeLib.FFITILE_ORIGIN_SECTION_Z_OFFSET);
        var visibleSectionsPtr = tilePtr + HyperCullNativeLib.FFITILE_VISIBLE_SECTIONS_OFFSET;

        // We assume that tile X and Z coordinates line up with region X and Z coordinates.
        int regionX = tileSectionX >> RenderRegion.REGION_WIDTH_SH;
        int regionZ = tileSectionZ >> RenderRegion.REGION_LENGTH_SH;

        // Iterate regions on Y axis inside current tile. Regions and Tiles aren't guaranteed to align on the Y axis.
        SectionIteration.iterateSplitsOnAxis(
                tileSectionY,
                tileSectionY + TILE_HEIGHT,
                RenderRegion.REGION_HEIGHT,
                (regionY, minSectionYInRegion, maxSectionYInRegion, nextYInTile) -> {
                    var region = ((RegionAccess) this.regions).hypercull$get(regionX, regionY, regionZ);

                    if (region == null) {
                        return;
                    }

                    // Iterate over section Y levels in the region, while also keeping track of the Y level in the tile
                    // to fetch the visible section data.
                    long sectionYInTile = nextYInTile;
                    for (int sectionYInRegion = minSectionYInRegion; sectionYInRegion < maxSectionYInRegion; sectionYInRegion++) {
                        // Get 64 bits of visible sections at a time, which represents a full slice on the X and Z axes.
                        long visibleSectionsSlice =
                                MemoryUtil.memGetLong(visibleSectionsPtr + (sectionYInTile * Long.BYTES));
                        sectionYInTile++;

                        // Each 1-bit represents a section that is visible. Use Lemire-style set-bit iteration approach,
                        // found here: https://lemire.me/blog/2018/02/21/iterating-over-set-bits-quickly/. This will
                        // quickly skip over 0-bits.
                        while (visibleSectionsSlice != 0) {
                            // The index of the bit represents the ZX portion of the section's index inside the tile.
                            var bitIdx = Long.numberOfTrailingZeros(visibleSectionsSlice);
                            visibleSectionsSlice &= visibleSectionsSlice - 1;

                            // Bit indices are ordered as YZX in Tiles, but as XZY and Regions. We need to unpack the ZX
                            // portion of the tile index, which bitIdx represents, and then re-pack it to match the Region
                            // indexing scheme. Tiles and regions have the same size and alignment on the X and Z axis,
                            // so we can reuse section X and Z coordinates inside a tile for their respective region.
                            var sectionXInRegion = (bitIdx >> TILE_IDX_X_OFFSET) & TILE_WIDTH_M;
                            var sectionZInRegion = (bitIdx >> TILE_IDX_Z_OFFSET) & TILE_LENGTH_M;
                            var regionSectionIndex =  LocalSectionIndex.pack(
                                    sectionXInRegion,
                                    sectionYInRegion,
                                    sectionZInRegion
                            );

                            RenderSection section = region.getSection(regionSectionIndex);
                            if (section != null) {
                                section.setLastVisibleFrame(frame);
                                visitor.visit(section);
                            }
                        }
                    }
                }
        );
    }

    public void setSection(int x, int y, int z, long visibilityData) {
        HyperCullNativeLib.graphSetSection(
                this.nativePtr,
                x,
                y,
                z,
                visibilityData
        );
    }

    @Override
    public void close() {
        HyperCullNativeLib.graphDelete(this.nativePtr);
    }
}
