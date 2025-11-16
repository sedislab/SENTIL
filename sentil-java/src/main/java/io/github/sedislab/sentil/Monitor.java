package io.github.sedislab.sentil;

import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.OptionalLong;

/** A monitor for one formula. */
public final class Monitor extends NativeResource {
    Monitor(long handle) {
        super(handle, NativeLib::monitorDestroy);
    }

    /** A monitor for a formula, with the default discrete-time config. Consumes the formula. */
    public Monitor(Formula formula) throws SentilException {
        this(NativeLib.monitorCreate(formula.consume(), 0L));
    }

    /** A monitor for a formula with the given config. Consumes the formula. */
    public Monitor(Formula formula, Config config) throws SentilException {
        this(NativeLib.monitorCreate(formula.consume(), config.handle()));
    }

    /** A monitor for a formula string, with the default config. */
    public static Monitor parse(String formula) throws SentilException {
        return new Monitor(NativeLib.monitorParse(formula, 0L));
    }

    /** A monitor for a formula string with the given config. */
    public static Monitor parse(String formula, Config config) throws SentilException {
        return new Monitor(NativeLib.monitorParse(formula, config.handle()));
    }

    /** A copy of the monitored formula. */
    public Formula formula() throws SentilException {
        return new Formula(NativeLib.monitorFormula(handle()));
    }

    /** A copy of the monitor's config. */
    public Config config() throws SentilException {
        return new Config(NativeLib.monitorConfig(handle()));
    }

    /** The robustness over the trace, honoring the config's time mode. */
    public double robustness(Trace trace) throws SentilException {
        return NativeLib.monitorRobustness(handle(), trace.handle());
    }

    /** The robustness at every sample of the trace. */
    public double[] robustnessSignal(Trace trace) throws SentilException {
        return NativeLib.monitorRobustnessSignal(handle(), trace.handle());
    }

    /** The time spans where the property does not hold. */
    public List<Interval> violations(Trace trace) throws SentilException {
        return Arrays.asList(NativeLib.monitorViolations(handle(), trace.handle()));
    }

    /** The index of a variable in packed-update order, or empty if the formula skips it. */
    public OptionalLong symbolIndex(String name) throws SentilException {
        long[] index = NativeLib.monitorSymbolIndex(handle(), name);
        return index.length == 0 ? OptionalLong.empty() : OptionalLong.of(index[0]);
    }

    /** Fold one timestamped sample given as a map from variable name to value. */
    public Robustness update(double time, Map<String, Double> values) throws SentilException {
        NamedSample sample = NamedSample.of(values);
        return NativeLib.monitorUpdate(handle(), time, sample.names, sample.values);
    }

    /** Fold one sample with values already in {@link #symbolIndex} order. */
    public Robustness updatePacked(double time, double[] values) throws SentilException {
        return NativeLib.monitorUpdatePacked(handle(), time, values);
    }

    /** Clear streaming state so the monitor can run a fresh trace. */
    public void reset() {
        NativeLib.monitorReset(handle());
    }

    /** Check this monitor's probabilistic formula against the lifted trace ensemble. */
    public SmcResult check(Trace trace, LiftingRegistry lifting) throws SentilException {
        return NativeLib.monitorCheck(handle(), trace.handle(), lifting.handle());
    }

    /** Decide this monitor's probabilistic formula sequentially with Wald's SPRT. */
    public SprtResult checkSequential(Trace trace, LiftingRegistry lifting, SprtConfig config)
            throws SentilException {
        return NativeLib.monitorCheckSequential(handle(), trace.handle(), lifting.handle(),
                config.p0, config.p1, config.alpha, config.beta, config.maxSamples, config.seed);
    }
}