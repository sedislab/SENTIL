package io.github.sedislab.sentil;

/** How sensor noise combines with the true signal, additive y - g or multiplicative y / g. */
public enum NoiseInteraction {
    ADDITIVE(0),
    MULTIPLICATIVE(1);

    private final int code;

    NoiseInteraction(int code) {
        this.code = code;
    }

    int code() {
        return code;
    }
}