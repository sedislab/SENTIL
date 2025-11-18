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
}