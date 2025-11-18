package io.github.sedislab.sentil;

import java.util.Arrays;
import java.util.List;

/**
 * A sampling-ready stochastic system, the form the rare-event estimator consumes.
 * Build one from a {@link SimModel} with {@link SimModel#toStochasticSystem()}. A
 * system owns a native handle, so close it when done.
 */
public final class StochasticSystem extends NativeResource {
    StochasticSystem(long handle) {
        super(handle, NativeLib::stochasticSystemDestroy);
    }

    /** Simulate one full-horizon trajectory from a seed. */
    public Trace simulate(long seed) throws SentilException {
        return new Trace(NativeLib.stochasticSystemSimulate(handle(), seed));
    }

    /** The state variable names. */
    public List<String> variables() throws SentilException {
        return Arrays.asList(NativeLib.stochasticSystemVariables(handle()));
    }

    /** The time step. */
    public double dt() {
        return NativeLib.stochasticSystemDt(handle());
    }

    /** The number of steps in a trajectory. */
    public long horizon() {
        return NativeLib.stochasticSystemHorizon(handle());
    }
}