package io.github.sedislab.sentil;

/**
 * A batch objective for {@link Synthesis#cmaEsBatched} scoring a whole population at
 * once. Must be thread-safe.
 */
@FunctionalInterface
public interface BatchObjective {
    /** Score each point in points, returning one value per row in the same order. */
    double[] evaluate(double[][] points);
}