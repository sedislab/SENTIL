package io.github.sedislab.sentil;

/** The best point an optimizer found and the objective value there. */
public final class Optimum {
    private final double[] point;
    private final double value;

    Optimum(double[] point, double value) {
        this.point = point;
        this.value = value;
    }

    /** The best point. */
    public double[] point() {
        return point;
    }

    /** The objective value at the best point. */
    public double value() {
        return value;
    }
}