package io.github.sedislab.sentil;

/** The rollout of a custom {@link SystemModel}. Must be thread-safe. */
@FunctionalInterface
public interface Rollout {
    /** The trajectory as [variable][sample] with horizon + 1 samples per variable. */
    double[][] rollout(double[] initial, double[] input);
}