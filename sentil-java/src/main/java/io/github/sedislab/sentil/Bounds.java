package io.github.sedislab.sentil;

/** Box bounds on a synthesis decision vector. */
public final class Bounds extends NativeResource {
    Bounds(long handle) {
        super(handle, NativeLib::boundsDestroy);
    }

    /** Bounds with the given lower and upper limit per coordinate. */
    public Bounds(double[] lower, double[] upper) throws SentilException {
        this(NativeLib.boundsCreate(lower, upper));
    }

    /** Bounds that constrain nothing over the given number of coordinates. */
    public static Bounds unbounded(long dimension) throws SentilException {
        return new Bounds(NativeLib.boundsUnbounded(dimension));
    }

    /** The number of coordinates. */
    public long dimension() {
        return NativeLib.boundsDimension(handle());
    }

    /** The per-coordinate lower limits. */
    public double[] lower() {
        return NativeLib.boundsLower(handle());
    }

    /** The per-coordinate upper limits. */
    public double[] upper() {
        return NativeLib.boundsUpper(handle());
    }

    /** Project a point into the box, returning the clamped copy. */
    public double[] clamp(double[] point) {
        return NativeLib.boundsClamp(handle(), point);
    }
}