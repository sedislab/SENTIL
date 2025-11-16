package io.github.sedislab.sentil;

import java.util.Arrays;
import java.util.List;
import java.util.Optional;
import java.util.OptionalDouble;

/** A fixed-capacity ring buffer of timed samples with running statistics. */
public final class RingBuffer extends NativeResource {
    RingBuffer(long handle) {
        super(handle, NativeLib::ringBufferDestroy);
    }

    /** A buffer that holds at most capacity samples. */
    public static RingBuffer create(long capacity) throws SentilException {
        return new RingBuffer(NativeLib.ringBufferCreate(capacity));
    }

    /** Append a sample, returning the evicted oldest one on overflow. */
    public Optional<Sample> push(double time, double value) throws SentilException {
        Sample evicted = NativeLib.ringBufferPush(handle(), time, value);
        return evicted.found() ? Optional.of(evicted) : Optional.empty();
    }

    /** Drop every sample, keeping the capacity. */
    public void clear() {
        NativeLib.ringBufferClear(handle());
    }

    /** The number of samples currently held. */
    public long length() {
        return NativeLib.ringBufferLen(handle());
    }

    /** The most samples the buffer can hold. */
    public long capacity() {
        return NativeLib.ringBufferCapacity(handle());
    }

    public boolean isEmpty() {
        return NativeLib.ringBufferIsEmpty(handle());
    }

    public boolean isFull() {
        return NativeLib.ringBufferIsFull(handle());
    }

    /** The oldest sample. */
    public Sample front() {
        return NativeLib.ringBufferFront(handle());
    }

    /** The newest sample. */
    public Sample back() {
        return NativeLib.ringBufferBack(handle());
    }

    /** The sample at an index counted from the oldest. */
    public Sample get(long index) {
        return NativeLib.ringBufferGet(handle(), index);
    }

    /** Remove and return the oldest sample. */
    public Sample popFront() {
        return NativeLib.ringBufferPopFront(handle());
    }

    /** Remove and return the newest sample. */
    public Sample popBack() {
        return NativeLib.ringBufferPopBack(handle());
    }

    /** The held sample whose time is nearest the query. */
    public Sample closestToTime(double time) {
        return NativeLib.ringBufferClosestToTime(handle(), time);
    }

    private static OptionalDouble optional(double[] result) {
        return result.length == 0 ? OptionalDouble.empty() : OptionalDouble.of(result[0]);
    }

    /** The running mean of the buffered values. */
    public OptionalDouble mean() {
        return optional(NativeLib.ringBufferMean(handle()));
    }

    /** The running variance, empty until two samples are held. */
    public OptionalDouble variance() {
        return optional(NativeLib.ringBufferVariance(handle()));
    }

    /** The running standard deviation, empty until two samples are held. */
    public OptionalDouble stdDev() {
        return optional(NativeLib.ringBufferStdDev(handle()));
    }

    /** The smallest buffered value. */
    public OptionalDouble min() {
        return optional(NativeLib.ringBufferMin(handle()));
    }

    /** The largest buffered value. */
    public OptionalDouble max() {
        return optional(NativeLib.ringBufferMax(handle()));
    }

    /** Recompute the running mean and variance from scratch. */
    public void recomputeStatistics() {
        NativeLib.ringBufferRecomputeStatistics(handle());
    }

    /** The value recorded at the query time, within a small tolerance. */
    public OptionalDouble atTime(double time) {
        return optional(NativeLib.ringBufferAtTime(handle(), time));
    }

    /** The earliest and latest times held, as [start, end]. */
    public Optional<double[]> timeRange() {
        double[] range = NativeLib.ringBufferTimeRange(handle());
        return range.length == 0 ? Optional.empty() : Optional.of(range);
    }

    /** The samples whose time lies in [start, end], oldest first. */
    public List<Sample> between(double start, double end) throws SentilException {
        return Arrays.asList(NativeLib.ringBufferBetween(handle(), start, end));
    }
}