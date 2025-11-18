package io.github.sedislab.sentil;

/** The outcome of a rare-event estimate. */
public final class RareEventResult {
    private final double probability;
    private final double violationProbability;
    private final boolean holds;
    private final long simulations;

    RareEventResult(double probability, double violationProbability, boolean holds,
            long simulations) {
        this.probability = probability;
        this.violationProbability = violationProbability;
        this.holds = holds;
        this.simulations = simulations;
    }

    /** The estimated satisfaction probability. */
    public double probability() {
        return probability;
    }

    /** The estimated violation probability. */
    public double violationProbability() {
        return violationProbability;
    }

    /** Whether the probabilistic threshold is met. */
    public boolean holds() {
        return holds;
    }

    /** The number of trajectory simulations the estimate cost. */
    public long simulations() {
        return simulations;
    }

    @Override
    public String toString() {
        return "RareEventResult{probability=" + probability + ", holds=" + holds + ", simulations="
                + simulations + "}";
    }
}