package io.github.sedislab.sentil;

/** A statistical check with the robustness distribution across the sampled ensemble. */
public final class SmcDistribution {
    private final SmcResult result;
    private final RobustnessDistribution distribution;

    SmcDistribution(SmcResult result, RobustnessDistribution distribution) {
        this.result = result;
        this.distribution = distribution;
    }

    /** The check result. */
    public SmcResult result() {
        return result;
    }

    /** The robustness distribution across the ensemble. */
    public RobustnessDistribution distribution() {
        return distribution;
    }
}