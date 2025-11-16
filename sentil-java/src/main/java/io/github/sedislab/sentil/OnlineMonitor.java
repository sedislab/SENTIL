package io.github.sedislab.sentil;

import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.OptionalLong;

/** The streaming monitor, O(1) amortized per sample and memory proportional to the window. */
public final class OnlineMonitor extends NativeResource {
    OnlineMonitor(long handle) {
        super(handle, NativeLib::streamMonitorDestroy);
    }

    /** A streaming monitor for a formula string. */
    public static OnlineMonitor create(String formula) throws SentilException {
        return new OnlineMonitor(NativeLib.streamMonitorCreate(formula));
    }

    /** A streaming monitor for a formula, which is borrowed. */
    public static OnlineMonitor fromFormula(Formula formula) throws SentilException {
        return new OnlineMonitor(NativeLib.streamMonitorFromFormula(formula.handle()));
    }

    /**
     * A streaming monitor for a P~p(phi) formula, lifting each reading through the
     * registry. The formula and registry are borrowed.
     */
    public static OnlineMonitor withLifting(Formula formula, LiftingRegistry lifting,
            SmcConfig config) throws SentilException {
        return new OnlineMonitor(NativeLib.streamMonitorWithLifting(formula.handle(),
                lifting.handle(), config.samples, config.confidence, config.seed,
                config.method.code()));
    }

    /** The number of variables the formula reads. */
    public long variableCount() {
        return NativeLib.streamMonitorVariableCount(handle());
    }

    /** The index of a variable in packed-update order, or empty if the formula skips it. */
    public OptionalLong symbolIndex(String name) throws SentilException {
        long[] index = NativeLib.streamMonitorSymbolIndex(handle(), name);
        return index.length == 0 ? OptionalLong.empty() : OptionalLong.of(index[0]);
    }

    /** Fold one timestamped sample given as a map from variable name to value. */
    public Robustness update(double time, Map<String, Double> values) throws SentilException {
        NamedSample sample = NamedSample.of(values);
        return NativeLib.streamMonitorUpdate(handle(), time, sample.names, sample.values);
    }

    /** Fold one sample with values already in {@link #symbolIndex} order. */
    public Robustness updatePacked(double time, double[] values) throws SentilException {
        return NativeLib.streamMonitorUpdatePacked(handle(), time, values);
    }

    /** Replay a whole trace, returning the per-sample robustness. */
    public List<Robustness> run(Trace trace) throws SentilException {
        return Arrays.asList(NativeLib.streamMonitorRun(handle(), trace.handle()));
    }

    /** Clear streaming state so the monitor can run a fresh trace. */
    public void reset() {
        NativeLib.streamMonitorReset(handle());
    }
}