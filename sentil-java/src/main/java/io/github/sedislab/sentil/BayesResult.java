package io.github.sedislab.sentil;

/** The outcome of a Bayesian sequential test. */
public final class BayesResult {
    private final BayesVerdict verdict;
    private final long samples;
    private final double posterior;

    BayesResult(int verdict, long samples, double posterior) {
        this.verdict = BayesVerdict.fromCode(verdict);
        this.samples = samples;
        this.posterior = posterior;
    }

    public BayesVerdict verdict() {
        return verdict;
    }

    /** The number of samples drawn before the test stopped. */
    public long samples() {
        return samples;
    }

    /** The posterior probability the property holds. */
    public double posterior() {
        return posterior;
    }

    @Override
    public String toString() {
        return "BayesResult{verdict=" + verdict + ", samples=" + samples + ", posterior=" + posterior
                + "}";
    }
}