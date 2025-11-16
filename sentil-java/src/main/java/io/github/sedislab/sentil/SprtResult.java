package io.github.sedislab.sentil;

/** The outcome of a sequential probability ratio test. */
public final class SprtResult {
    private final SprtVerdict verdict;
    private final long samples;
    private final double logLikelihood;

    SprtResult(int verdict, long samples, double logLikelihood) {
        this.verdict = SprtVerdict.fromCode(verdict);
        this.samples = samples;
        this.logLikelihood = logLikelihood;
    }

    public SprtVerdict verdict() {
        return verdict;
    }

    /** The number of samples drawn before the test stopped. */
    public long samples() {
        return samples;
    }

    public double logLikelihood() {
        return logLikelihood;
    }

    @Override
    public String toString() {
        return "SprtResult{verdict=" + verdict + ", samples=" + samples + "}";
    }
}