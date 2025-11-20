package io.github.sedislab.sentil;

/** The step callback of a custom {@link StochasticSystem}. Must be thread-safe. */
@FunctionalInterface
public interface SystemStep {
    /** The next state. */
    double[] step(double[] previous, double time, long seed);
}