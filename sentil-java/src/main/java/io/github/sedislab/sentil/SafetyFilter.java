package io.github.sedislab.sentil;

/** A least-restrictive shield over a nominal input. */
public final class SafetyFilter extends NativeResource {
    /** A filter enforcing the given bounds, which it consumes. */
    public SafetyFilter(Bounds bounds) throws SentilException {
        super(NativeLib.safetyFilterCreate(bounds.consume()), NativeLib::safetyFilterDestroy);
    }

    /** The input closest to nominal that stays inside the bounds. */
    public double[] filter(double[] nominal) throws SentilException {
        return NativeLib.safetyFilterFilter(handle(), nominal, new double[0], new double[0], 0);
    }

    /**
     * The input closest to nominal that satisfies the bounds and each barrier
     * {@code barrierA[i] . u >= barrierB[i]}.
     */
    public double[] filter(double[] nominal, double[][] barrierA, double[] barrierB)
            throws SentilException {
        int m = barrierA.length;
        int n = nominal.length;
        double[] flat = new double[m * n];
        for (int i = 0; i < m; i++) {
            System.arraycopy(barrierA[i], 0, flat, i * n, n);
        }
        return NativeLib.safetyFilterFilter(handle(), nominal, flat, barrierB, m);
    }
}