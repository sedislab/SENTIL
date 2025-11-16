package io.github.sedislab.sentil;

/** Summary statistics of robustness across the sampled ensemble. */
public final class RobustnessDistribution {
    private final long count;
    private final double mean;
    private final double variance;
    private final double stdDev;
    private final double min;
    private final double max;

    RobustnessDistribution(long count, double mean, double variance, double stdDev, double min,
            double max) {
        this.count = count;
        this.mean = mean;
        this.variance = variance;
        this.stdDev = stdDev;
        this.min = min;
        this.max = max;
    }

    public long count() {
        return count;
    }

    public double mean() {
        return mean;
    }

    public double variance() {
        return variance;
    }

    public double stdDev() {
        return stdDev;
    }

    public double min() {
        return min;
    }

    public double max() {
        return max;
    }

    @Override
    public String toString() {
        return "RobustnessDistribution{count=" + count + ", mean=" + mean + ", stdDev=" + stdDev
                + ", min=" + min + ", max=" + max + "}";
    }
}