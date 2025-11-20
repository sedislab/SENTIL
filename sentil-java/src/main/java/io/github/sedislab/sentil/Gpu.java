package io.github.sedislab.sentil;

/** The GPU capability check. */
public final class Gpu {
    private Gpu() {
    }

    /** Whether a usable GPU device is present. */
    public static boolean isAvailable() {
        return NativeLib.gpuIsAvailable();
    }
}