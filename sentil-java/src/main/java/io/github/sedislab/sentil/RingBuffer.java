package io.github.sedislab.sentil;

import java.util.Optional;

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
}