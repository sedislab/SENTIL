package io.github.sedislab.sentil;

/** Entry points for the SENTIL Java binding. */
public final class Sentil {
    private Sentil() {
    }

    /** The version of the native engine this binding wraps. */
    public static Version version() {
        int[] v = NativeLib.version();
        return new Version(v[0], v[1], v[2]);
    }
}