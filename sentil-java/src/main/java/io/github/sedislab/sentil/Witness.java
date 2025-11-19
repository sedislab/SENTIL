package io.github.sedislab.sentil;

/** A witnessing run from the counterexample or falsification search. */
public final class Witness implements AutoCloseable {
    private final double[] input;
    private final double robustness;
    private final Trace trace;

    Witness(double[] input, double robustness, long trace) {
        this.input = input;
        this.robustness = robustness;
        this.trace = new Trace(trace);
    }

    /** The input sequence that produced this run. */
    public double[] input() {
        return input;
    }

    /** The robustness the run achieved. */
    public double robustness() {
        return robustness;
    }

    /** The trace the run drove. */
    public Trace trace() {
        return trace;
    }

    /** Free the trace this witness owns. */
    @Override
    public void close() {
        trace.close();
    }
}