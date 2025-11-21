package io.github.sedislab.sentil;

import java.util.Arrays;
import java.util.List;

/** The per-variable noise models that lift a deterministic trace to a stochastic ensemble. */
public final class LiftingRegistry extends NativeResource {
    LiftingRegistry(long handle) {
        super(handle, NativeLib::liftingDestroy);
    }

    public LiftingRegistry() throws SentilException {
        this(NativeLib.liftingCreate());
    }

    /** Attach a noise model to a variable. The model is consumed. */
    public void register(String variable, NoiseModel model, NoiseInteraction interaction)
            throws SentilException {
        long registry = handle();
        long modelHandle = model.consume();
        NativeLib.liftingRegister(registry, variable, modelHandle, interaction.code());
    }

    /** Attach a noise model to a variable with the default additive interaction. */
    public void register(String variable, NoiseModel model) throws SentilException {
        register(variable, model, NoiseInteraction.ADDITIVE);
    }

    /** The variables that carry a noise model, sorted. */
    public List<String> variables() throws SentilException {
        return Arrays.asList(NativeLib.liftingVariables(handle()));
    }

    /** Whether no variable carries a noise model. */
    public boolean isEmpty() {
        return NativeLib.liftingIsEmpty(handle());
    }

    /** One seeded noisy realization of the trace. */
    public Trace lift(Trace trace, long seed) throws SentilException {
        return new Trace(NativeLib.liftingLift(handle(), trace.handle(), seed));
    }
}