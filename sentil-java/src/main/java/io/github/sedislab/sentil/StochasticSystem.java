package io.github.sedislab.sentil;

import java.util.Arrays;
import java.util.List;

/** A sampling-ready stochastic system, the form the rare-event estimator consumes. */
public final class StochasticSystem extends NativeResource {
    private final long box;

    StochasticSystem(long handle) {
        super(handle, NativeLib::stochasticSystemDestroy);
        this.box = 0L;
    }

    private StochasticSystem(long handle, long box) {
        super(handle, h -> {
            NativeLib.stochasticSystemDestroy(h);
            NativeLib.freeSystemBox(box);
        });
        this.box = box;
    }

    /** A system whose dynamics are host callbacks. They must be thread-safe. */
    public static StochasticSystem custom(String[] variables, double dt, long horizon,
            SystemInit init, SystemStep step) throws SentilException {
        long[] result = NativeLib.stochasticSystemCreateCustom(variables, dt, horizon, init, step);
        return new StochasticSystem(result[0], result[1]);
    }

    /** Simulate one full-horizon trajectory from a seed. */
    public Trace simulate(long seed) throws SentilException {
        Trace trace = new Trace(NativeLib.stochasticSystemSimulate(handle(), seed));
        try {
            rethrowCallbackError();
        } catch (RuntimeException e) {
            trace.close();
            throw e;
        }
        return trace;
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

    void rethrowCallbackError() {
        if (box != 0L) {
            NativeLib.rethrowSystemError(box);
        }
    }
}