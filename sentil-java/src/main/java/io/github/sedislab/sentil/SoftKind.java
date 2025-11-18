package io.github.sedislab.sentil;

/** The soft min and max used by smooth robustness. */
public enum SoftKind {
    LOG_SUM_EXP(0),
    ARITHMETIC_GEOMETRIC_MEAN(1);

    private final int code;

    SoftKind(int code) {
        this.code = code;
    }

    int code() {
        return code;
    }
}