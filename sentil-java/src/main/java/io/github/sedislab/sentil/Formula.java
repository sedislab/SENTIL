package io.github.sedislab.sentil;

import java.util.Arrays;
import java.util.List;

/** A parsed PrSTL formula. The combinators consume their operands. */
public final class Formula extends NativeResource {
    Formula(long handle) {
        super(handle, NativeLib::formulaDestroy);
    }

    /** Parse a PrSTL formula such as {@code "always[0, 10](x > 0)"}. */
    public static Formula parse(String formula) throws SentilException {
        return new Formula(NativeLib.formulaParse(formula));
    }

    /** Rebuild a formula from JSON. */
    public static Formula fromJson(String json) throws SentilException {
        return new Formula(NativeLib.formulaFromJson(json));
    }

    /** The JSON form of this formula. */
    public String toJson() throws SentilException {
        return NativeLib.formulaToJson(handle());
    }

    /** The nesting depth, a predicate being 1. */
    public long depth() {
        return NativeLib.formulaDepth(handle());
    }

    /** Whether the formula uses any temporal operator. */
    public boolean hasTemporal() {
        return NativeLib.formulaHasTemporal(handle());
    }

    /** The variable names the formula reads, sorted and unique. */
    public List<String> variables() throws SentilException {
        return Arrays.asList(NativeLib.formulaVariables(handle()));
    }

    /** The robustness of the formula over a trace, reading the sample grid. */
    public double robustness(Trace trace) throws SentilException {
        return NativeLib.formulaRobustness(handle(), trace.handle());
    }

    /** The robustness over a trace, catching threshold crossings between samples. */
    public double robustnessDense(Trace trace) throws SentilException {
        return NativeLib.formulaRobustnessDense(handle(), trace.handle());
    }

    /** The robustness at every sample of the trace, reading the grid. */
    public double[] robustnessSignal(Trace trace) throws SentilException {
        return NativeLib.formulaRobustnessSignal(handle(), trace.handle());
    }

    /** The dense robustness at every sample of the trace. */
    public double[] robustnessDenseSignal(Trace trace) throws SentilException {
        return NativeLib.formulaRobustnessDenseSignal(handle(), trace.handle());
    }
}