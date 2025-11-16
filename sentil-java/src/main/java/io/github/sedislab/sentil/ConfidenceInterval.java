package io.github.sedislab.sentil;

/** A binomial proportion confidence interval at a confidence level. */
public final class ConfidenceInterval {
    private final double lower;
    private final double upper;
    private final double level;

    ConfidenceInterval(double lower, double upper, double level) {
        this.lower = lower;
        this.upper = upper;
        this.level = level;
    }

    public double lower() {
        return lower;
    }

    public double upper() {
        return upper;
    }

    /** The confidence level, for instance 0.95. */
    public double level() {
        return level;
    }

    /** The width of the interval. */
    public double width() {
        return upper - lower;
    }

    @Override
    public String toString() {
        return "[" + lower + ", " + upper + "] @ " + level;
    }
}