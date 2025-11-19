package io.github.sedislab.sentil;

/** An objective for {@link Synthesis#maximize} returning its value and gradient at a point. */
@FunctionalInterface
public interface GradientObjective {
    /** Compute the objective value at x and write the gradient into gradient. */
    double evaluate(double[] x, double[] gradient);
}