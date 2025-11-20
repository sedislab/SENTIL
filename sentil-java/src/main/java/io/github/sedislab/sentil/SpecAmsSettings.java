package io.github.sedislab.sentil;

/** The rare-event splitting settings a specification recommends. */
public final class SpecAmsSettings {
    private final long numParticles;
    private final long maxSteps;

    SpecAmsSettings(long numParticles, long maxSteps) {
        this.numParticles = numParticles;
        this.maxSteps = maxSteps;
    }

    public long numParticles() {
        return numParticles;
    }

    public long maxSteps() {
        return maxSteps;
    }
}