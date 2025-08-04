package com.github.burgerindividual.hypercull.client.ffi;

import com.github.burgerindividual.hypercull.client.HyperCullClientMod;
import com.github.burgerindividual.hypercull.client.FrustumPlaneProvider;
import net.caffeinemc.mods.sodium.client.render.viewport.CameraTransform;
import org.lwjgl.system.*;

import oshi.SystemInfo;

public class HyperCullNativeLib {
    public static final boolean SUPPORTED;

    // Size and alignment information obtained by hovering over Rust types with Rust Analyzer.
    public static final int FFITILE_SIZE = 80;
    public static final int FFITILE_ORIGIN_SECTION_X_OFFSET = 0;
    public static final int FFITILE_ORIGIN_SECTION_Y_OFFSET = 4;
    public static final int FFITILE_ORIGIN_SECTION_Z_OFFSET = 8;
    public static final int FFITILE_VISIBLE_SECTIONS_OFFSET = align(12, Pointer.POINTER_SIZE);

    public static final int FFISLICE_ALIGNMENT = Pointer.POINTER_SIZE;
    public static final int FFISLICE_SIZE = Pointer.POINTER_SIZE * 2;
    public static final int FFISLICE_DATA_PTR_OFFSET = 0;
    public static final int FFISLICE_COUNT_OFFSET = Pointer.POINTER_SIZE;

    public static final int FFICAMERA_SIZE = 120;
    public static final int FFICAMERA_ALIGNMENT = Pointer.POINTER_SIZE;
    public static final int FFICAMERA_FRUSTUM_PLANES_OFFSET = 24;

    public static final int FRUSTUM_PLANE_SIZE = 16;
    public static final int FRUSTUM_PLANE_ALIGNMENT = 4;

    private static final PanicCallback PANIC_CALLBACK;

    static {
        var errorLoading = false;
        PanicCallback panicCallback = null;

        try {
            var architecture = Platform.getArchitecture();
            var systemType = String.format(
                    "%s-%s%s",
                    Platform.get().getName().toLowerCase(),
                    architecture.name().toLowerCase(),
                    getCPUFeatures(architecture)
            );
            var nativePath = String.format(
                    "assets/hypercull/natives/%s/%s",
                    systemType,
                    System.mapLibraryName("hypercull_native")
            );

            Library.loadSystem(
                    System::load,
                    System::loadLibrary,
                    HyperCullNativeLib.class,
                    "",
                    nativePath
            );

            panicCallback = initPanicHandler();

            HyperCullClientMod.LOGGER.info(
                    "Native culling library initialized successfully (loaded from {})",
                    nativePath
            );
        } catch (Throwable t) {
            HyperCullClientMod.LOGGER.error("Error loading native culling library", t);
            errorLoading = true;
        }

        SUPPORTED = !errorLoading;
        PANIC_CALLBACK = panicCallback;
    }

    public static void init() {
        // Calling this function will make sure the class is loaded and the static initializer is run.
    }

    private static String getCPUFeatures(Platform.Architecture architecture) {
        if (architecture.equals(Platform.Architecture.X64)) {
            var cpuFeatureStrings = new SystemInfo().getHardware().getProcessor().getFeatureFlags();

            // Windows does not let us check for the presence of FMA in its API, so we'll just assume it's present if
            // AVX2 is present. Are there any CPUs where this isn't the case?
            var hasAVX2 = false;
            var hasSSE41 = false;
            var hasSSSE3 = false;

            for (var cpuFeatureString : cpuFeatureStrings) {
                var lowercaseFeatureString = cpuFeatureString.toLowerCase();
                hasAVX2 |= lowercaseFeatureString.contains("avx2");
                hasSSE41 |= lowercaseFeatureString.contains("sse4_1") || lowercaseFeatureString.contains("sse4.1");
                hasSSSE3 |= lowercaseFeatureString.contains("ssse3");
            }

            if (hasAVX2) {
                return "-avx2+fma";
            } else if (hasSSE41 && hasSSSE3) {
                return "-sse4_1+ssse3";
            }
        }

        return "";
    }

    private static PanicCallback initPanicHandler() {
        var panicCallback = PanicCallback.defaultHandler();
        setPanicHandler(panicCallback.address());
        return panicCallback;
    }

    public static void freePanicHandler() {
        if (PANIC_CALLBACK != null) {
            PANIC_CALLBACK.free();
        }
    }

    public static long frustumCreate(
            MemoryStack stack,
            FrustumPlaneProvider frustum,
            CameraTransform transform
    ) {
        long pCamera = stack.nmalloc(FFICAMERA_ALIGNMENT, FFICAMERA_SIZE);

        var planes = frustum.hypercull$getPlanes();

        MemoryUtil.memPutDouble(pCamera, transform.x);
        MemoryUtil.memPutDouble(pCamera + 8, transform.y);
        MemoryUtil.memPutDouble(pCamera + 16, transform.z);

        long pFrustumPlanes = stack.nmalloc(FRUSTUM_PLANE_ALIGNMENT, FRUSTUM_PLANE_SIZE * planes.length);

        for (int planeIdx = 0; planeIdx < planes.length; planeIdx++) {
            long planeOffsetBytes = (long) planeIdx * 16;
            MemoryUtil.memPutFloat(pFrustumPlanes + planeOffsetBytes, planes[planeIdx].x);
            MemoryUtil.memPutFloat(pFrustumPlanes + planeOffsetBytes + 4, planes[planeIdx].y);
            MemoryUtil.memPutFloat(pFrustumPlanes + planeOffsetBytes + 8, planes[planeIdx].z);
            MemoryUtil.memPutFloat(pFrustumPlanes + planeOffsetBytes + 12, planes[planeIdx].w);
        }

        MemoryUtil.memPutAddress(pCamera + FFICAMERA_FRUSTUM_PLANES_OFFSET + FFISLICE_DATA_PTR_OFFSET, pFrustumPlanes);
        MemoryUtil.memPutAddress(pCamera + FFICAMERA_FRUSTUM_PLANES_OFFSET + FFISLICE_COUNT_OFFSET, planes.length);

        return pCamera;
    }

    /**
     * @param panic_handler_fn_ptr Rust Type: {@code PanicHandlerFn}
     */
    private static native void setPanicHandler(long panic_handler_fn_ptr);

    /**
     * @param render_distance        Rust Type: {@code u8}
     * @param world_bottom_section_y Rust Type: {@code i8}
     * @param world_top_section_y    Rust Type: {@code i8}
     * @return a native pointer to a Graph instance allocated with the system allocator.
     *                               Rust Type: {@code *mut Graph}
     */
    public static native long graphCreate(byte render_distance, byte world_bottom_section_y, byte world_top_section_y);

    /**
     * @param graph_ptr              Rust Type: {@code *mut Graph}
     * @param x                      Rust Type: {@code i32}
     * @param y                      Rust Type: {@code i32}
     * @param z                      Rust Type: {@code i32}
     * @param visibility_bitmask     Rust Type: {@code u64}
     */
    public static native void graphSetSection(long graph_ptr, int x, int y, int z, long visibility_bitmask);

    /**
     * @param return_value_ptr      Rust Type: {@code *mut FFISlice<FFIVisibleSectionsTile>}
     * @param graph_ptr             Rust Type: {@code *mut Graph}
     * @param camera_ptr            Rust Type: {@code *const FFICamera}
     * @param search_distance       Rust Type: {@code f32}
     * @param use_occlusion_culling Rust Type: {@code bool}
     */
    public static native void graphSearch(long return_value_ptr, long graph_ptr, long camera_ptr, float search_distance, boolean use_occlusion_culling);

    /**
     * @param graph_ptr Rust Type: {@code *mut Graph}
     */
    public static native void graphDelete(long graph_ptr);

    /**
     * <p>Rounds the integer {@param num} up to the next multiple of {@param alignment}. This multiple *MUST* be
     * a power-of-two, or undefined behavior will occur.</p>
     *
     * @param num The number to round up
     * @param alignment The power-of-two multiple to round to
     * @return The rounded number
     */
    private static int align(int num, int alignment) {
        int additive = alignment - 1;
        int mask = ~additive;
        return (num + additive) & mask;
    }
}
