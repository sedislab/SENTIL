package io.github.sedislab.sentil;

import java.util.Arrays;
import java.util.List;
import java.util.Optional;

/** One or more named signals sampled over a strictly increasing sequence of times. */
public final class Trace extends NativeResource {
    private Trace(long handle) {
        super(handle, NativeLib::traceDestroy);
    }

    /** A trace over the given times, with no signals yet. */
    public static Trace create(double[] times) throws SentilException {
        return new Trace(NativeLib.traceCreate(times));
    }

    /** A trace holding one named signal over the given times. */
    public static Trace fromSignal(double[] times, String name, double[] values)
            throws SentilException {
        return new Trace(NativeLib.traceFromSignal(times, name, values));
    }

    /** A trace with integer times 0, 1, ..., length - 1 and no signals yet. */
    public static Trace indexed(long length) throws SentilException {
        return new Trace(NativeLib.traceIndexed(length));
    }

    /** Add or replace a named signal, whose length must equal the trace length. */
    public void addSignal(String name, double[] values) throws SentilException {
        NativeLib.traceAddSignal(handle(), name, values);
    }

    /** The number of time points. */
    public long length() {
        return NativeLib.traceLen(handle());
    }

    /** Whether the trace has no time points. */
    public boolean isEmpty() {
        return NativeLib.traceIsEmpty(handle());
    }

    /** A copy of the sample times. */
    public double[] times() {
        return NativeLib.traceTimes(handle());
    }

    /** The names of the signals the trace carries, sorted. */
    public List<String> variables() throws SentilException {
        return Arrays.asList(NativeLib.traceVariables(handle()));
    }

    /** A copy of a named signal's values, or empty if the trace has no such signal. */
    public Optional<double[]> signal(String name) {
        return Optional.ofNullable(NativeLib.traceSignal(handle(), name));
    }
}