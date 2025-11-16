package io.github.sedislab.sentil;

/** The verdict of a Bayesian sequential test. */
public enum BayesVerdict {
    HOLDS(0),
    FAILS(1),
    INCONCLUSIVE(2);

    private final int code;

    BayesVerdict(int code) {
        this.code = code;
    }

    static BayesVerdict fromCode(int code) {
        for (BayesVerdict value : values()) {
            if (value.code == code) {
                return value;
            }
        }
        return INCONCLUSIVE;
    }
}