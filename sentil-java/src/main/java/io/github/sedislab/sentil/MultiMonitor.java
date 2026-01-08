package io.github.sedislab.sentil;

import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.OptionalDouble;

/** Several streaming formulas under one clock. */
public final class MultiMonitor extends NativeResource {
    public MultiMonitor() throws SentilException {
        super(NativeLib.multiMonitorCreate(), NativeLib::multiMonitorDestroy);
    }

    /** Add a formula string under an id. */
    public void add(String id, String formula) throws SentilException {
        NativeLib.multiMonitorAdd(handle(), id, formula);
    }

    /** Add a formula under an id; the formula is borrowed, not consumed. */
    public void add(String id, Formula formula) throws SentilException {
        NativeLib.multiMonitorAddFormula(handle(), id, formula.handle());
    }

    /** Remove the first formula with the id, returning whether one was found. */
    public boolean remove(String id) {
        return NativeLib.multiMonitorRemove(handle(), id);
    }

    /** Clear every monitor's streaming state. */
    public void reset() {
        NativeLib.multiMonitorReset(handle());
    }

    /** The number of formulas. */
    public long size() {
        return NativeLib.multiMonitorLen(handle());
    }

    /** Whether no formula is registered. */
    public boolean isEmpty() {
        return NativeLib.multiMonitorIsEmpty(handle());
    }

    /** The ids in insertion order. */
    public List<String> ids() throws SentilException {
        return Arrays.asList(NativeLib.multiMonitorIds(handle()));
    }

    /**
     * Add a P~p(phi) formula tracked online through a lifted particle ensemble. The
     * formula and registry are borrowed.
     */
    public void addProbabilistic(String id, Formula formula, LiftingRegistry lifting,
            SmcConfig config) throws SentilException {
        NativeLib.multiMonitorAddProbabilistic(handle(), id, formula.handle(), lifting.handle(),
                config.samples, config.confidence, config.seed, config.method.code());
    }

    /** Advance every monitor at this sample, returning the verdict for each id. */
    public Map<String, Robustness> update(double time, Map<String, Double> values)
            throws SentilException {
        NamedSample sample = NamedSample.of(values);
        return NativeLib.multiMonitorUpdate(handle(), time, sample.names, sample.values);
    }

    /** The last satisfaction probability for the formula under {@code id}, or empty. */
    public OptionalDouble probability(String id) {
        double p = NativeLib.multiMonitorProbability(handle(), id);
        return Double.isNaN(p) ? OptionalDouble.empty() : OptionalDouble.of(p);
    }

    /** Each formula's last probability keyed by id, in insertion order. */
    public Map<String, OptionalDouble> probabilities() throws SentilException {
        Map<String, OptionalDouble> out = new LinkedHashMap<>();
        for (String id : ids()) {
            out.put(id, probability(id));
        }
        return out;
    }
}