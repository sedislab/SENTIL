package io.github.sedislab.sentil;

/** Rare-event splitting settings. */
public final class RareEventConfig {
    long particles = 4096;
    double margin = 0.0;
    long seed = 42;

    public long particles() {
        return particles;
    }

    public RareEventConfig particles(long particles) {
        this.particles = particles;
        return this;
    }

    public double margin() {
        return margin;
    }

    public RareEventConfig margin(double margin) {
        this.margin = margin;
        return this;
    }

    public long seed() {
        return seed;
    }

    public RareEventConfig seed(long seed) {
        this.seed = seed;
        return this;
    }
}