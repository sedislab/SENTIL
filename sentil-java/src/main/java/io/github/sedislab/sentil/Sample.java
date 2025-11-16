package io.github.sedislab.sentil;

/** A timed sample. */
public final class Sample {
    private final boolean found;
    private final double time;
    private final double value;

    Sample(boolean found, double time, double value) {
        this.found = found;
        this.time = time;
        this.value = value;
    }

    /** Whether the query found a sample. */
    public boolean found() {
        return found;
    }

    public double time() {
        return time;
    }

    public double value() {
        return value;
    }

    @Override
    public String toString() {
        return found ? "Sample{time=" + time + ", value=" + value + "}" : "Sample{none}";
    }
}