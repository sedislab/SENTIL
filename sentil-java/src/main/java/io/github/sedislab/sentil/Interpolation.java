package io.github.sedislab.sentil;

/** How a trace fills values between its samples when resampling. */
public enum Interpolation {
    LINEAR(0),
    ZERO_ORDER_HOLD(1),
    CUBIC_SPLINE(2);

    private final int code;

    Interpolation(int code) {
        this.code = code;
    }

    int code() {
        return code;
    }
}