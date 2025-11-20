package io.github.sedislab.sentil;

/** Builds a formula for a parameter value, for {@link Synthesis#mineTightestParameter}. */
@FunctionalInterface
public interface ParameterFormula {
    /** The formula for a parameter value. The miner takes ownership of it. */
    Formula make(double param) throws SentilException;
}