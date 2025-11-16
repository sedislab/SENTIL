package io.github.sedislab.sentil;

/** The threshold direction of a probabilistic operator P~p(phi). */
public enum ProbabilityOp {
    GE(0),
    GT(1),
    LE(2),
    LT(3);

    private final int code;

    ProbabilityOp(int code) {
        this.code = code;
    }

    int code() {
        return code;
    }
}