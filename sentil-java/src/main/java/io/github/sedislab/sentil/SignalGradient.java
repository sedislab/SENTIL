package io.github.sedislab.sentil;

/** The smooth robustness over a trace and its gradient at every signal sample. */
public final class SignalGradient {
    private final double value;
    private final double[][] gradient;

    SignalGradient(double value, double[][] gradient) {
        this.value = value;
        this.gradient = gradient;
    }

    /** The smooth robustness. */
    public double value() {
        return value;
    }

    /** The gradient as [variable][sample], in sorted variable order. */
    public double[][] gradient() {
        return gradient;
    }
}