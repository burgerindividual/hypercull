package com.github.burgerindividual.hypercull.mixin.iris;

import net.irisshaders.iris.shadows.frustum.BoxCuller;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

@Mixin(BoxCuller.class)
public interface BoxCullerAccessor {
    @Accessor
    double getMaxDistance();
}
