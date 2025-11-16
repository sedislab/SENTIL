package io.github.sedislab.sentil;

/** A time span [start, end] where a property does not hold. */
public final class Interval {
    private final double start;
    private final double end;

    Interval(double start, double end) {
        this.start = start;
        this.end = end;
    }

    public double start() {
        return start;
    }

    public double end() {
        return end;
    }

    @Override
    public String toString() {
        return "[" + start + ", " + end + "]";
    }
}