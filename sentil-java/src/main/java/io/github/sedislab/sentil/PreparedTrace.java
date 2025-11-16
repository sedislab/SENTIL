package io.github.sedislab.sentil;

/** A trace with its interpolation coefficients fixed once. */
public final class PreparedTrace extends NativeResource {
    PreparedTrace(long handle) {
        super(handle, NativeLib::preparedTraceDestroy);
    }

    /** Resample onto a new set of times. */
    public Trace resample(double[] times) throws SentilException {
        return new Trace(NativeLib.preparedTraceResample(handle(), times));
    }
}