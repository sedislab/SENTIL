package io.github.sedislab.sentil;

/** The smooth robustness of a trajectory and its gradient in each input coordinate. */
public final class InputGradient {
    private final double value;
    private final double[] gradient;

    InputGradient(double value, double[] gradient) {
        this.value = value;
        this.gradient = gradient;
    }

    /** The smooth robustness. */
    public double value() {
        return value;
    }

    /** The gradient with respect to each input coordinate. */
    public double[] gradient() {
        return gradient;
    }
}