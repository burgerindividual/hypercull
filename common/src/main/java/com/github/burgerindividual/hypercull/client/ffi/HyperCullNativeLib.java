package com.github.burgerindividual.hypercull.client.ffi;

import com.github.burgerindividual.hypercull.client.HyperCullClientMod;
import com.github.burgerindividual.hypercull.client.SixPlaneFrustum;
import net.caffeinemc.mods.sodium.client.render.viewport.CameraTransform;
import org.lwjgl.system.*;

import oshi.SystemInfo;

import static org.joml.FrustumIntersection.*;

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
                    System.mapLibraryName("hypercull")
            );

            Library.loadSystem(
                    System::load,
                    System::loadLibrary,
                    HyperCullNativeLib.class,
                    "",
                    nativePath
            );

            panicCallback = initPanicHandler();
        } catch (Throwable t) {
            HyperCullClientMod.LOGGER.error("Error loading native culling library", t);
            errorLoading = true;
        }

        SUPPORTED = !errorLoading;
        PANIC_CALLBACK = panicCallback;
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
            SixPlaneFrustum frustum,
            CameraTransform transform
    ) {
        long pFrustum = stack.nmalloc(FFICAMERA_ALIGNMENT, FFICAMERA_SIZE);

        var planes = frustum.hypercull$getPlanes();

        // the order of the planes in memory matches the direction order used in the native code
        // (NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z)
        MemoryUtil.memPutFloat(pFrustum, planes[PLANE_NX].x);
        MemoryUtil.memPutFloat(pFrustum + 4, planes[PLANE_NX].y);
        MemoryUtil.memPutFloat(pFrustum + 8, planes[PLANE_NX].z);
        MemoryUtil.memPutFloat(pFrustum + 12, planes[PLANE_NX].w);

        MemoryUtil.memPutFloat(pFrustum + 16, planes[PLANE_NY].x);
        MemoryUtil.memPutFloat(pFrustum + 20, planes[PLANE_NY].y);
        MemoryUtil.memPutFloat(pFrustum + 24, planes[PLANE_NY].z);
        MemoryUtil.memPutFloat(pFrustum + 28, planes[PLANE_NY].w);

        MemoryUtil.memPutFloat(pFrustum + 32, planes[PLANE_NZ].x);
        MemoryUtil.memPutFloat(pFrustum + 36, planes[PLANE_NZ].y);
        MemoryUtil.memPutFloat(pFrustum + 40, planes[PLANE_NZ].z);
        MemoryUtil.memPutFloat(pFrustum + 44, planes[PLANE_NZ].w);

        MemoryUtil.memPutFloat(pFrustum + 48, planes[PLANE_PX].x);
        MemoryUtil.memPutFloat(pFrustum + 52, planes[PLANE_PX].y);
        MemoryUtil.memPutFloat(pFrustum + 56, planes[PLANE_PX].z);
        MemoryUtil.memPutFloat(pFrustum + 60, planes[PLANE_PX].w);

        MemoryUtil.memPutFloat(pFrustum + 64, planes[PLANE_PY].x);
        MemoryUtil.memPutFloat(pFrustum + 68, planes[PLANE_PY].y);
        MemoryUtil.memPutFloat(pFrustum + 72, planes[PLANE_PY].z);
        MemoryUtil.memPutFloat(pFrustum + 76, planes[PLANE_PY].w);

        MemoryUtil.memPutFloat(pFrustum + 80, planes[PLANE_PZ].x);
        MemoryUtil.memPutFloat(pFrustum + 84, planes[PLANE_PZ].y);
        MemoryUtil.memPutFloat(pFrustum + 88, planes[PLANE_PZ].z);
        MemoryUtil.memPutFloat(pFrustum + 92, planes[PLANE_PZ].w);

        MemoryUtil.memPutDouble(pFrustum + 96, transform.x);
        MemoryUtil.memPutDouble(pFrustum + 104, transform.y);
        MemoryUtil.memPutDouble(pFrustum + 112, transform.z);

        return pFrustum;
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
