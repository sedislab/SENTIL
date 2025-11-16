package io.github.sedislab.sentil;

/** Whether a monitor reads the sample grid (discrete) or crossings between samples (dense). */
public enum TimeMode {
    DISCRETE(0),
    DENSE(1);

    private final int code;

    TimeMode(int code) {
        this.code = code;
    }

    int code() {
        return code;
    }

    static TimeMode fromCode(int code) {
        return code == DENSE.code ? DENSE : DISCRETE;
    }
}