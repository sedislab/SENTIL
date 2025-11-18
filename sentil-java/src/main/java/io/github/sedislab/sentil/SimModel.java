package io.github.sedislab.sentil;

import java.util.Arrays;
import java.util.List;

/** A declarative stochastic model, one init and one advance expression per variable. */
public final class SimModel extends NativeResource {
    SimModel(long handle) {
        super(handle, NativeLib::simModelDestroy);
    }

    /**
     * Build a model over the named variables stepping by dt for horizon steps. The
     * init, advance, and noise arrays are consumed.
     */
    public static SimModel create(String[] variables, double dt, long horizon, SimExpr[] init,
            SimExpr[] advance, NoiseModel[] noise) throws SentilException {
        long[] initHandles = collect(init);
        long[] advanceHandles = collect(advance);
        long[] noiseHandles = collect(noise);
        disownAll(init);
        disownAll(advance);
        disownAll(noise);
        return new SimModel(NativeLib.simModelCreate(variables, dt, horizon, initHandles,
                advanceHandles, noiseHandles));
    }

    private static long[] collect(NativeResource[] resources) {
        long[] handles = new long[resources.length];
        for (int i = 0; i < resources.length; i++) {
            handles[i] = resources[i].handle();
        }
        return handles;
    }

    private static void disownAll(NativeResource[] resources) {
        for (NativeResource resource : resources) {
            resource.disown();
        }
    }

    /** Simulate one full-horizon trajectory from a seed. */
    public Trace simulate(long seed) throws SentilException {
        return new Trace(NativeLib.simModelSimulate(handle(), seed));
    }

    /** The state variable names. */
    public List<String> variables() throws SentilException {
        return Arrays.asList(NativeLib.simModelVariables(handle()));
    }

    /** The time step. */
    public double dt() {
        return NativeLib.simModelDt(handle());
    }

    /** The number of steps in a trajectory. */
    public long horizon() {
        return NativeLib.simModelHorizon(handle());
    }

    /** Convert to a stochastic system for the CPU rare-event path. */
    public StochasticSystem toStochasticSystem() throws SentilException {
        return new StochasticSystem(NativeLib.simModelToStochasticSystem(handle()));
    }
}