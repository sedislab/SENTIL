package io.github.sedislab.sentil;

/** Bayesian sequential test settings. */
public final class BayesConfig {
    double threshold;
    double bayesFactor = 100.0;
    long maxSamples = 100000;
    long seed = 42;

    public BayesConfig(double threshold) {
        this.threshold = threshold;
    }

    public double threshold() {
        return threshold;
    }

    public double bayesFactor() {
        return bayesFactor;
    }

    public BayesConfig bayesFactor(double bayesFactor) {
        this.bayesFactor = bayesFactor;
        return this;
    }

    public long maxSamples() {
        return maxSamples;
    }

    public BayesConfig maxSamples(long maxSamples) {
        this.maxSamples = maxSamples;
        return this;
    }

    public long seed() {
        return seed;
    }

    public BayesConfig seed(long seed) {
        this.seed = seed;
        return this;
    }
}