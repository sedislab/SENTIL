package io.github.sedislab.sentil;

/** The statistical model checking settings a specification recommends. */
public final class SpecSmcSettings {
    private final double confidence;
    private final long sampleBudget;

    SpecSmcSettings(double confidence, long sampleBudget) {
        this.confidence = confidence;
        this.sampleBudget = sampleBudget;
    }

    public double confidence() {
        return confidence;
    }

    public long sampleBudget() {
        return sampleBudget;
    }
}