package io.github.sedislab.sentil;

/** CMA-ES settings for the falsifier and the black-box backend. */
public final class CmaConfig {
    long population = 0;
    long maxGenerations = 300;
    double initialStep = 0.3;
    double tolStep = 1e-11;
    long seed = 42;

    public long population() {
        return population;
    }

    public CmaConfig population(long population) {
        this.population = population;
        return this;
    }

    public long maxGenerations() {
        return maxGenerations;
    }

    public CmaConfig maxGenerations(long maxGenerations) {
        this.maxGenerations = maxGenerations;
        return this;
    }

    public double initialStep() {
        return initialStep;
    }

    public CmaConfig initialStep(double initialStep) {
        this.initialStep = initialStep;
        return this;
    }

    public double tolStep() {
        return tolStep;
    }

    public CmaConfig tolStep(double tolStep) {
        this.tolStep = tolStep;
        return this;
    }

    public long seed() {
        return seed;
    }

    public CmaConfig seed(long seed) {
        this.seed = seed;
        return this;
    }
}