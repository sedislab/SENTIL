package io.github.sedislab.sentil;

/** The comparison in a predicate f(x) ~ c. */
public enum ComparisonOp {
    LT(0),
    LE(1),
    GT(2),
    GE(3),
    EQ(4),
    NE(5);

    private final int code;

    ComparisonOp(int code) {
        this.code = code;
    }

    int code() {
        return code;
    }
}