package io.github.sedislab.sentil;

/** The sequential probability ratio test settings a specification recommends. */
public final class SpecSprtSettings {
    private final double p0;
    private final double p1;
    private final double alpha;
    private final double beta;
    private final long maxSamples;

    SpecSprtSettings(double p0, double p1, double alpha, double beta, long maxSamples) {
        this.p0 = p0;
        this.p1 = p1;
        this.alpha = alpha;
        this.beta = beta;
        this.maxSamples = maxSamples;
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

    public double beta() {
        return beta;
    }

    public long maxSamples() {
        return maxSamples;
    }
}