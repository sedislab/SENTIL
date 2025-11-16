package io.github.sedislab.sentil;

/** A robustness verdict. */
public final class Robustness {
    private final boolean resolved;
    private final boolean satisfied;
    private final double value;
    private final double lower;
    private final double upper;

    Robustness(boolean resolved, boolean satisfied, double value, double lower, double upper) {
        this.resolved = resolved;
        this.satisfied = satisfied;
        this.value = value;
        this.lower = lower;
        this.upper = upper;
    }

    /** Whether the verdict is final. */
    public boolean resolved() {
        return resolved;
    }

    /** Whether the property holds. */
    public boolean satisfied() {
        return satisfied;
    }

    /** The robustness, or the interval midpoint while unresolved. */
    public double value() {
        return value;
    }

    /** The lower bound while unresolved. */
    public double lower() {
        return lower;
    }

    /** The upper bound while unresolved. */
    public double upper() {
        return upper;
    }

    @Override
    public String toString() {
        return "Robustness{resolved=" + resolved + ", satisfied=" + satisfied + ", value=" + value
                + ", lower=" + lower + ", upper=" + upper + "}";
    }
}