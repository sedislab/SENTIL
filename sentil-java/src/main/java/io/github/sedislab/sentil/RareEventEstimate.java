package io.github.sedislab.sentil;

/** A rare-event probability estimate from a user-defined simulator. */
public final class RareEventEstimate {
    private final double probability;
    private final long simulations;

    RareEventEstimate(double probability, long simulations) {
        this.probability = probability;
        this.simulations = simulations;
    }

    /** The estimated rare-event probability. */
    public double probability() {
        return probability;
    }

    /** The number of trajectory simulations the estimate cost. */
    public long simulations() {
        return simulations;
    }

    @Override
    public String toString() {
        return "RareEventEstimate{probability=" + probability + ", simulations=" + simulations + "}";
    }
}