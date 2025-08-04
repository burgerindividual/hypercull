package com.github.burgerindividual.hypercull.mixin;

import com.github.burgerindividual.hypercull.client.HyperCullClientMod;
import com.llamalad7.mixinextras.injector.ModifyExpressionValue;
import net.minecraft.client.gui.components.DebugScreenOverlay;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;

import java.util.ArrayList;

@Mixin(DebugScreenOverlay.class)
public class MixinDebugScreenOverlay {
    // TODO: clean up or disable in production
    @ModifyExpressionValue(
        method = "getSystemInformation",
        at = @At(
            value = "INVOKE",
            target = "Lcom/google/common/collect/Lists;newArrayList([Ljava/lang/Object;)Ljava/util/ArrayList;"
        )
    )
    private ArrayList<String> addDebugStrings(ArrayList<String> strings) {
        strings.add("");
        strings.add("HC Invocations: " + HyperCullClientMod.FRAME_INVOCATIONS);
        strings.add("HC Fallbacks: " + HyperCullClientMod.FRAME_FALLBACKS);
        HyperCullClientMod.FRAME_INVOCATIONS = 0;
        HyperCullClientMod.FRAME_FALLBACKS = 0;

        return strings;
    }
}
