package io.github.sedislab.sentil;

/** Sequential probability ratio test settings, requiring 0 &lt; p0 &lt; p1 &lt; 1. */
public final class SprtConfig {
    double p0;
    double p1;
    double alpha = 0.05;
    double beta = 0.05;
    long maxSamples = 100000;
    long seed = 42;

    public SprtConfig(double p0, double p1) {
        this.p0 = p0;
        this.p1 = p1;
    }

    public double p0() {
        return p0;
    }

    public double p1() {
        return p1;
    }

    public double alpha() {
        return alpha;
    }

    public SprtConfig alpha(double alpha) {
        this.alpha = alpha;
        return this;
    }

    public double beta() {
        return beta;
    }

    public SprtConfig beta(double beta) {
        this.beta = beta;
        return this;
    }

    public long maxSamples() {
        return maxSamples;
    }

    public SprtConfig maxSamples(long maxSamples) {
        this.maxSamples = maxSamples;
        return this;
    }

    public long seed() {
        return seed;
    }

    public SprtConfig seed(long seed) {
        this.seed = seed;
        return this;
    }
}