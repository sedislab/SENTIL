package io.github.sedislab.sentil;

import java.util.List;
import java.util.function.ToDoubleFunction;

/** The smooth-robustness primitives, the synthesis numerics, and the black-box optimizers. */
public final class Synthesis {
    private Synthesis() {
    }

    /** The smooth, differentiable minimum of the values at the given temperature. */
    public static double softMin(double[] values, double temperature) {
        return NativeLib.softMin(values, temperature);
    }

    /** The smooth, differentiable maximum of the values at the given temperature. */
    public static double softMax(double[] values, double temperature) {
        return NativeLib.softMax(values, temperature);
    }

    /** Find an input sequence for the model that best satisfies the spec. */
    public static SynthesisResult synthesize(SystemModel model, Formula spec)
            throws SentilException {
        return synthesize(model, spec, null, null, Backend.AUTO, 0, 0);
    }

    /**
     * Find an input sequence for the model that best satisfies the spec. bounds and
     * smooth may be null, and maxIters and population of 0 take the defaults.
     */
    public static SynthesisResult synthesize(SystemModel model, Formula spec, Bounds bounds,
            SmoothConfig smooth, Backend backend, long maxIters, long population)
            throws SentilException {
        long boundsHandle = bounds == null ? 0L : bounds.handle();
        double temperature = smooth == null ? 0.0 : smooth.temperature;
        int kind = smooth == null ? 0 : smooth.kind.code();
        SynthesisResult result = NativeLib.synthesize(model.handle(), spec.handle(), boundsHandle,
                temperature, kind, smooth != null, maxIters, backend.code(), population);
        model.rethrowCallbackError();
        return result;
    }

    /** Minimize 1/2 u'Pu + q'u subject to Gu &lt;= h, with P symmetric positive-definite. */
    public static double[] solveQp(double[][] p, double[] q, double[][] g, double[] h, long maxIters)
            throws SentilException {
        int n = p.length;
        int m = g.length;
        return NativeLib.solveQp(Matrices.flatten(p, n), n, q, Matrices.flatten(g, n), m, h, maxIters);
    }

    /** Solve Ax = b for a symmetric positive-definite A. */
    public static double[] solveSpd(double[][] matrix, double[] rhs) throws SentilException {
        int n = matrix.length;
        return NativeLib.solveSpd(Matrices.flatten(matrix, n), n, rhs);
    }

    /** The eigendecomposition of a symmetric matrix. */
    public static EigenDecomposition symmetricEigen(double[][] matrix) throws SentilException {
        int n = matrix.length;
        double[] packed = NativeLib.symmetricEigen(Matrices.flatten(matrix, n), n);
        double[] values = new double[n];
        System.arraycopy(packed, 0, values, 0, n);
        double[][] vectors = new double[n][n];
        for (int i = 0; i < n; i++) {
            System.arraycopy(packed, n + i * n, vectors[i], 0, n);
        }
        return new EigenDecomposition(values, vectors);
    }

    private static Optimum optimum(double[] packed) {
        double[] point = new double[packed.length - 1];
        System.arraycopy(packed, 1, point, 0, point.length);
        return new Optimum(point, packed[0]);
    }

    /** Maximize a gradient objective from start, optionally inside bounds. */
    public static Optimum maximize(GradientObjective objective, double[] start, Bounds bounds,
            long maxIters) throws SentilException {
        if (bounds == null) {
            try (Bounds unbounded = Bounds.unbounded(start.length)) {
                return optimum(NativeLib.maximize(objective, start, unbounded.handle(), maxIters));
            }
        }
        return optimum(NativeLib.maximize(objective, start, bounds.handle(), maxIters));
    }

    /**
     * Maximize a scalar objective with gradient-free CMA-ES from start, optionally
     * inside bounds.
     */
    public static Optimum cmaEs(ToDoubleFunction<double[]> objective, double[] start, Bounds bounds,
            CmaConfig config) throws SentilException {
        if (bounds == null) {
            try (Bounds unbounded = Bounds.unbounded(start.length)) {
                return cmaEs(objective, start, unbounded, config);
            }
        }
        return optimum(NativeLib.cmaEs(objective, start, bounds.handle(), config.population,
                config.maxGenerations, config.initialStep, config.tolStep, config.seed));
    }

    /** Maximize a batch objective with CMA-ES, scoring a whole population at once. */
    public static Optimum cmaEsBatched(BatchObjective objective, double[] start, Bounds bounds,
            CmaConfig config) throws SentilException {
        if (bounds == null) {
            try (Bounds unbounded = Bounds.unbounded(start.length)) {
                return cmaEsBatched(objective, start, unbounded, config);
            }
        }
        return optimum(NativeLib.cmaEsBatched(objective, start, bounds.handle(), config.population,
                config.maxGenerations, config.initialStep, config.tolStep, config.seed));
    }

    /**
     * The tightest parameter in [lower, upper] for which the formula make builds holds
     * on every trace.
     */
    public static double mineTightestParameter(ParameterFormula make, List<Trace> traces,
            double lower, double upper) throws SentilException {
        long[] handles = new long[traces.size()];
        for (int i = 0; i < handles.length; i++) {
            handles[i] = traces.get(i).handle();
        }
        return NativeLib.mineTightestParameter(make, handles, lower, upper);
    }

    /**
     * Estimate a rare-event probability over a user-defined simulator by adaptive
     * multilevel splitting.
     */
    public static RareEventEstimate adaptiveMultilevelSplitting(AmsInterface simulator,
            long particles, double targetScore, long maxSteps, long seed) throws SentilException {
        return NativeLib.adaptiveMultilevelSplitting(simulator, particles, targetScore, maxSteps,
                seed);
    }
}