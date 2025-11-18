package io.github.sedislab.sentil;

/** Smooth-robustness settings. */
public final class SmoothConfig {
    double temperature = 10.0;
    SoftKind kind = SoftKind.LOG_SUM_EXP;

    public double temperature() {
        return temperature;
    }

    public SmoothConfig temperature(double temperature) {
        this.temperature = temperature;
        return this;
    }

    public SoftKind kind() {
        return kind;
    }

    public SmoothConfig kind(SoftKind kind) {
        this.kind = kind;
        return this;
    }
}