package io.github.sedislab.sentil;

/**
 * The smooth-robustness primitives and synthesis numerics the synthesizer, the
 * controller, and the witness search build on. All methods are static; there is
 * nothing to construct.
 */
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
        return NativeLib.synthesize(model.handle(), spec.handle(), boundsHandle, temperature, kind,
                smooth != null, maxIters, backend.code(), population);
    }
}