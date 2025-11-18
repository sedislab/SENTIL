package io.github.sedislab.sentil;

/** The outcome of validating a chance constraint. */
public final class ChanceReport {
    private final double estimate;
    private final double lowerBound;
    private final long samples;
    private final boolean holds;

    ChanceReport(double estimate, double lowerBound, long samples, boolean holds) {
        this.estimate = estimate;
        this.lowerBound = lowerBound;
        this.samples = samples;
        this.holds = holds;
    }

    /** The point estimate of the satisfaction probability. */
    public double estimate() {
        return estimate;
    }

    /** The confidence lower bound on the probability. */
    public double lowerBound() {
        return lowerBound;
    }

    /** The number of sampled trajectories. */
    public long samples() {
        return samples;
    }

    /** Whether the constraint is met. */
    public boolean holds() {
        return holds;
    }

    @Override
    public String toString() {
        return "ChanceReport{estimate=" + estimate + ", lowerBound=" + lowerBound + ", holds="
                + holds + "}";
    }
}