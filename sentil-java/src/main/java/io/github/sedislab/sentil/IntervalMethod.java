package io.github.sedislab.sentil;

/** The binomial confidence interval estimator. */
public enum IntervalMethod {
    WILSON(0),
    CLOPPER_PEARSON(1),
    JEFFREYS(2),
    AGRESTI_COULL(3);

    private final int code;

    IntervalMethod(int code) {
        this.code = code;
    }

    int code() {
        return code;
    }
}