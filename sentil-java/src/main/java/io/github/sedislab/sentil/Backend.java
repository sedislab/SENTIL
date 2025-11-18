package io.github.sedislab.sentil;

/** The optimization backend open-loop synthesis chooses or is told to use. */
public enum Backend {
    AUTO(0),
    GRADIENT(1),
    CMA_ES(2),
    MILP(3);

    private final int code;

    Backend(int code) {
        this.code = code;
    }

    int code() {
        return code;
    }

    static Backend fromCode(int code) {
        for (Backend value : values()) {
            if (value.code == code) {
                return value;
            }
        }
        return AUTO;
    }
}