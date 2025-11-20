package io.github.sedislab.sentil;

/** The initial-state callback of a custom {@link StochasticSystem}. Must be thread-safe. */
@FunctionalInterface
public interface SystemInit {
    /** The initial state for a seed, as one value per state variable. */
    double[] init(long seed);
}