package io.github.sedislab.sentil;

/** Statistical model checking settings. */
public final class SmcConfig {
    long samples = 10000;
    double confidence = 0.95;
    long seed = 42;
    IntervalMethod method = IntervalMethod.WILSON;

    public long samples() {
        return samples;
    }

    public SmcConfig samples(long samples) {
        this.samples = samples;
        return this;
    }

    public double confidence() {
        return confidence;
    }

    public SmcConfig confidence(double confidence) {
        this.confidence = confidence;
        return this;
    }

    public long seed() {
        return seed;
    }

    public SmcConfig seed(long seed) {
        this.seed = seed;
        return this;
    }

    public IntervalMethod method() {
        return method;
    }

    public SmcConfig method(IntervalMethod method) {
        this.method = method;
        return this;
    }
}