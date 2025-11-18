package io.github.sedislab.sentil;

import java.util.function.BooleanSupplier;

/** The statistical primitives behind probabilistic monitoring. */
public final class Stats {
    private Stats() {
    }

    /** The Wilson score interval for successes out of trials at a confidence level. */
    public static ConfidenceInterval wilson(long successes, long trials, double level) {
        return NativeLib.wilsonInterval(successes, trials, level);
    }

    /** The Clopper-Pearson exact interval. */
    public static ConfidenceInterval clopperPearson(long successes, long trials, double level) {
        return NativeLib.clopperPearson(successes, trials, level);
    }

    /** The Jeffreys interval, with a Beta(1/2, 1/2) prior. */
    public static ConfidenceInterval jeffreys(long successes, long trials, double level) {
        return NativeLib.jeffreysInterval(successes, trials, level);
    }

    /** The Agresti-Coull interval. */
    public static ConfidenceInterval agrestiCoull(long successes, long trials, double level) {
        return NativeLib.agrestiCoull(successes, trials, level);
    }

    /** The interval from the default Wilson estimator. */
    public static ConfidenceInterval interval(long successes, long trials, double level) {
        return interval(IntervalMethod.WILSON, successes, trials, level);
    }

    /** The interval from the chosen estimator. */
    public static ConfidenceInterval interval(IntervalMethod method, long successes, long trials,
            double level) {
        return NativeLib.intervalByMethod(method.code(), successes, trials, level);
    }

    /** The two-sided z critical value for a confidence level in (0, 1). */
    public static double zScore(double level) {
        return NativeLib.zScore(level);
    }

    /** The Chernoff-Hoeffding sample count for a target absolute error and confidence. */
    public static long chernoffHoeffdingSamples(double epsilon, double delta)
            throws SentilException {
        return NativeLib.chernoffHoeffdingSamples(epsilon, delta);
    }

    /** The sample count for a target Wilson half-width at a confidence level. */
    public static long wilsonSamples(double epsilon, double level) throws SentilException {
        return NativeLib.wilsonSamples(epsilon, level);
    }

    /** Run Wald's SPRT over a caller-supplied Bernoulli source. */
    public static SprtResult sequentialTest(SprtConfig config, BooleanSupplier draw)
            throws SentilException {
        return NativeLib.sequentialTest(config.p0, config.p1, config.alpha, config.beta,
                config.maxSamples, config.seed, draw);
    }

    /** Run the Bayesian sequential test over a caller-supplied Bernoulli source. */
    public static BayesResult bayesSequentialTest(BayesConfig config, BooleanSupplier draw)
            throws SentilException {
        return NativeLib.bayesSequentialTest(config.threshold, config.bayesFactor, config.maxSamples,
                config.seed, draw);
    }
}