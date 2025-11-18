package io.github.sedislab.sentil;

/** The result of open-loop synthesis. */
public final class SynthesisResult {
    private final double[] input;
    private final double robustness;
    private final boolean holds;
    private final Backend backend;

    SynthesisResult(double[] input, double robustness, boolean holds, int backend) {
        this.input = input;
        this.robustness = robustness;
        this.holds = holds;
        this.backend = Backend.fromCode(backend);
    }

    /** The synthesized input sequence. */
    public double[] input() {
        return input;
    }

    /** The robustness the input achieves on the model. */
    public double robustness() {
        return robustness;
    }

    /** Whether the input satisfies the spec. */
    public boolean holds() {
        return holds;
    }

    /** The backend that produced this result. */
    public Backend backend() {
        return backend;
    }

    @Override
    public String toString() {
        return "SynthesisResult{robustness=" + robustness + ", holds=" + holds + ", backend="
                + backend + "}";
    }
}