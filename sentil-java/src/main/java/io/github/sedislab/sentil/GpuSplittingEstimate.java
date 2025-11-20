package io.github.sedislab.sentil;

/** A fixed-effort multilevel-splitting estimate from the GPU. */
public final class GpuSplittingEstimate {
    private final double violationProbability;
    private final long particles;
    private final int levels;

    GpuSplittingEstimate(double violationProbability, long particles, int levels) {
        this.violationProbability = violationProbability;
        this.particles = particles;
        this.levels = levels;
    }

    /** The estimated violation probability. */
    public double violationProbability() {
        return violationProbability;
    }

    /** The number of particles the splitter ran. */
    public long particles() {
        return particles;
    }

    /** The number of level thresholds it crossed. */
    public int levels() {
        return levels;
    }

    @Override
    public String toString() {
        return "GpuSplittingEstimate{violationProbability=" + violationProbability + ", particles="
                + particles + ", levels=" + levels + "}";
    }
}