package com.github.burgerindividual.hypercull.mixin;

import com.github.burgerindividual.hypercull.client.SixPlaneFrustum;
import com.github.burgerindividual.hypercull.client.NativeGraph;
import com.github.burgerindividual.hypercull.client.ffi.HyperCullNativeLib;
import com.llamalad7.mixinextras.injector.wrapoperation.Operation;
import com.llamalad7.mixinextras.injector.wrapoperation.WrapOperation;
import it.unimi.dsi.fastutil.longs.Long2ReferenceMap;
import net.caffeinemc.mods.sodium.client.gl.device.CommandList;
import net.caffeinemc.mods.sodium.client.render.chunk.RenderSection;
import net.caffeinemc.mods.sodium.client.render.chunk.RenderSectionManager;
import net.caffeinemc.mods.sodium.client.render.chunk.data.BuiltSectionInfo;
import net.caffeinemc.mods.sodium.client.render.chunk.occlusion.OcclusionCuller;
import net.caffeinemc.mods.sodium.client.render.chunk.region.RenderRegionManager;
import net.caffeinemc.mods.sodium.client.render.viewport.Viewport;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.world.level.Level;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(value = RenderSectionManager.class, remap = false)
public class MixinRenderSectionManager {
    @Shadow
    @Final
    private RenderRegionManager regions;

    @Unique
    private NativeGraph nativeGraph = null;

    @WrapOperation(
            method = "<init>",
            at = @At(value = "NEW", target = "net/caffeinemc/mods/sodium/client/render/chunk/occlusion/OcclusionCuller")
    )
    private OcclusionCuller skipJavaCullerCreation(
            Long2ReferenceMap<RenderSection> sections,
            Level level,
            Operation<OcclusionCuller> original
    ) {
        if (HyperCullNativeLib.SUPPORTED) {
            return null;
        } else {
            return original.call(sections, level);
        }
    }

    @Inject(method = "<init>", at = @At(value = "TAIL"))
    private void initNativeGraph(ClientLevel level, int renderDistance, CommandList commandList, CallbackInfo ci) {
        if (HyperCullNativeLib.SUPPORTED) {
            this.nativeGraph = new NativeGraph(
                    this.regions,
                    (byte) renderDistance,
                    (byte) level.getMinSectionY(),
                    (byte) level.getMaxSectionY()
            );
        }
    }

    @WrapOperation(
            method = "createTerrainRenderList",
            at = @At(value = "INVOKE", target = "Lnet/caffeinemc/mods/sodium/client/render/chunk/occlusion/OcclusionCuller;findVisible(Lnet/caffeinemc/mods/sodium/client/render/chunk/occlusion/OcclusionCuller$Visitor;Lnet/caffeinemc/mods/sodium/client/render/viewport/Viewport;FZI)V")
    )
    private void replaceCullOperation(
            OcclusionCuller instance,
            OcclusionCuller.Visitor visitor,
            Viewport viewport,
            float searchDistance,
            boolean useOcclusionCulling,
            int frame,
            Operation<Void> original
    ) {
        //noinspection ConstantValue
        if (HyperCullNativeLib.SUPPORTED
                && this.nativeGraph != null
                && ((ViewportAccessor) (Object) viewport).getFrustum() instanceof SixPlaneFrustum sixPlaneFrustum) {
            this.nativeGraph.findVisible(
                    visitor,
                    sixPlaneFrustum,
                    viewport.getTransform(),
                    searchDistance,
                    useOcclusionCulling,
                    frame
            );
        } else {
            original.call(instance, visitor, viewport, searchDistance, useOcclusionCulling, frame);
        }
    }

    @WrapOperation(
        method = "updateSectionInfo",
        at = @At(value = "INVOKE", target = "Lnet/caffeinemc/mods/sodium/client/render/chunk/RenderSection;setInfo(Lnet/caffeinemc/mods/sodium/client/render/chunk/data/BuiltSectionInfo;)Z")
    )
    private boolean wrapSectionUpdate(RenderSection render, BuiltSectionInfo info, Operation<Boolean> original) {
        var oldVisibilityData = render.getVisibilityData();
        var infoChanged = original.call(render, info);
        var newVisibilityData = render.getVisibilityData();

        if (HyperCullNativeLib.SUPPORTED && this.nativeGraph != null && oldVisibilityData != newVisibilityData) {
            this.nativeGraph.setSection(
                    render.getChunkX(),
                    render.getChunkY(),
                    render.getChunkZ(),
                    newVisibilityData
            );
        }

        return infoChanged;
    }

    @Inject(
        method = "destroy",
        at = @At(value = "TAIL")
    )
    private void destroyNativeGraph(CallbackInfo ci) {
        if (HyperCullNativeLib.SUPPORTED && this.nativeGraph != null) {
            this.nativeGraph.close();
        }
    }

//    /**
//     * @author burgerindividual
//     * @reason FOR TESTING PURPOSES ONLY, REMOVE FOR PRODUCTION
//     */
//    @Overwrite
//    public boolean needsUpdate() {
//        return true;
//    }
}
