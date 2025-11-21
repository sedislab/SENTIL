package io.github.sedislab.sentil;

import java.util.Arrays;
import java.util.List;
import java.util.Map;

/** A batch of named formulas evaluated together over one trace. */
public final class FormulaBank extends NativeResource {
    public FormulaBank() throws SentilException {
        super(NativeLib.formulaBankCreate(), NativeLib::formulaBankDestroy);
    }

    /** Add a formula string under an id. */
    public void add(String id, String formula) throws SentilException {
        NativeLib.formulaBankAdd(handle(), id, formula);
    }

    /** Add a formula under an id; the formula is borrowed, not consumed. */
    public void add(String id, Formula formula) throws SentilException {
        NativeLib.formulaBankAddFormula(handle(), id, formula.handle());
    }

    /** The ids in insertion order. */
    public List<String> ids() throws SentilException {
        return Arrays.asList(NativeLib.formulaBankIds(handle()));
    }

    /** The number of formulas. */
    public long size() {
        return NativeLib.formulaBankLen(handle());
    }

    /** Whether no formula is registered. */
    public boolean isEmpty() {
        return NativeLib.formulaBankIsEmpty(handle());
    }

    /** The robustness of every formula over the trace, keyed by id. */
    public Map<String, Double> robustness(Trace trace) throws SentilException {
        return NativeLib.formulaBankRobustness(handle(), trace.handle());
    }

    /** The dense-time robustness of every formula over the trace, keyed by id. */
    public Map<String, Double> robustnessDense(Trace trace) throws SentilException {
        return NativeLib.formulaBankRobustnessDense(handle(), trace.handle());
    }
}