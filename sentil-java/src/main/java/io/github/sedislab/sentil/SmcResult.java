package io.github.sedislab.sentil;

/** The outcome of a statistical check. */
public final class SmcResult {
    private final double probability;
    private final ConfidenceInterval interval;
    private final long satisfactions;
    private final long samples;
    private final boolean holds;

    SmcResult(double probability, ConfidenceInterval interval, long satisfactions, long samples,
            boolean holds) {
        this.probability = probability;
        this.interval = interval;
        this.satisfactions = satisfactions;
        this.samples = samples;
        this.holds = holds;
    }

    /** The empirical satisfaction probability. */
    public double probability() {
        return probability;
    }

    /** The confidence interval around the probability. */
    public ConfidenceInterval interval() {
        return interval;
    }

    /** The number of sampled trajectories that satisfied the formula. */
    public long satisfactions() {
        return satisfactions;
    }

    /** The number of sampled trajectories. */
    public long samples() {
        return samples;
    }

    /** Whether the probabilistic threshold is met. */
    public boolean holds() {
        return holds;
    }

    @Override
    public String toString() {
        return "SmcResult{probability=" + probability + ", interval=" + interval + ", holds=" + holds
                + "}";
    }
}