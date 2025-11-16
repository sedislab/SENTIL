package io.github.sedislab.sentil;

import java.util.OptionalDouble;

/** A noise distribution for stochastic signal lifting. */
public final class NoiseModel extends NativeResource {
    NoiseModel(long handle) {
        super(handle, NativeLib::noiseDestroy);
    }

    /** A point mass at value. */
    public static NoiseModel dirac(double value) throws SentilException {
        return new NoiseModel(NativeLib.noiseDirac(value));
    }

    /** A normal distribution. */
    public static NoiseModel gaussian(double mean, double stdDev) throws SentilException {
        return new NoiseModel(NativeLib.noiseGaussian(mean, stdDev));
    }

    /** A uniform distribution on [low, high]. */
    public static NoiseModel uniform(double low, double high) throws SentilException {
        return new NoiseModel(NativeLib.noiseUniform(low, high));
    }

    /** A log-normal distribution. */
    public static NoiseModel logNormal(double mu, double sigma) throws SentilException {
        return new NoiseModel(NativeLib.noiseLogNormal(mu, sigma));
    }

    /** An exponential distribution with the given rate. */
    public static NoiseModel exponential(double rate) throws SentilException {
        return new NoiseModel(NativeLib.noiseExponential(rate));
    }

    /** A gamma distribution. */
    public static NoiseModel gamma(double shape, double scale) throws SentilException {
        return new NoiseModel(NativeLib.noiseGamma(shape, scale));
    }

    /** A beta distribution. */
    public static NoiseModel beta(double alpha, double beta) throws SentilException {
        return new NoiseModel(NativeLib.noiseBeta(alpha, beta));
    }

    /** A Weibull distribution. */
    public static NoiseModel weibull(double shape, double scale) throws SentilException {
        return new NoiseModel(NativeLib.noiseWeibull(shape, scale));
    }

    /** A Rayleigh distribution. */
    public static NoiseModel rayleigh(double scale) throws SentilException {
        return new NoiseModel(NativeLib.noiseRayleigh(scale));
    }

    /** A Gumbel distribution. */
    public static NoiseModel gumbel(double location, double scale) throws SentilException {
        return new NoiseModel(NativeLib.noiseGumbel(location, scale));
    }

    /** A Cauchy distribution. */
    public static NoiseModel cauchy(double location, double scale) throws SentilException {
        return new NoiseModel(NativeLib.noiseCauchy(location, scale));
    }

    /** A Student's t distribution. */
    public static NoiseModel studentT(double df, double location, double scale)
            throws SentilException {
        return new NoiseModel(NativeLib.noiseStudentT(df, location, scale));
    }

    /** A normal distribution truncated to [lower, upper]. */
    public static NoiseModel truncatedNormal(double mean, double stdDev, double lower, double upper)
            throws SentilException {
        return new NoiseModel(NativeLib.noiseTruncatedNormal(mean, stdDev, lower, upper));
    }

    /** A Poisson distribution with the given rate. */
    public static NoiseModel poisson(double rate) throws SentilException {
        return new NoiseModel(NativeLib.noisePoisson(rate));
    }

    /** A binomial distribution of n trials with success probability p. */
    public static NoiseModel binomial(long n, double p) throws SentilException {
        return new NoiseModel(NativeLib.noiseBinomial(n, p));
    }

    /** An empirical model resampled from residuals. */
    public static NoiseModel bootstrap(double[] residuals) throws SentilException {
        return new NoiseModel(NativeLib.noiseBootstrap(residuals));
    }

    /** Rebuild a model from JSON. */
    public static NoiseModel fromJson(String json) throws SentilException {
        return new NoiseModel(NativeLib.noiseFromJson(json));
    }

    /** Load a model from a JSON file. */
    public static NoiseModel fromFile(String path) throws SentilException {
        return new NoiseModel(NativeLib.noiseFromFile(path));
    }

    /** The analytic mean, or empty where it is undefined. */
    public OptionalDouble mean() {
        double[] result = NativeLib.noiseMean(handle());
        return result.length == 0 ? OptionalDouble.empty() : OptionalDouble.of(result[0]);
    }

    /** The analytic variance, or empty where it is undefined. */
    public OptionalDouble variance() {
        double[] result = NativeLib.noiseVariance(handle());
        return result.length == 0 ? OptionalDouble.empty() : OptionalDouble.of(result[0]);
    }

    /** The model as a JSON string. */
    public String toJson() throws SentilException {
        return NativeLib.noiseToJson(handle());
    }
}