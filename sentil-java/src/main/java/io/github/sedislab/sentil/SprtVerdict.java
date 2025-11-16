package io.github.sedislab.sentil;

/** The verdict of a sequential probability ratio test. */
public enum SprtVerdict {
    ACCEPT_H0(0),
    ACCEPT_H1(1),
    INCONCLUSIVE(2);

    private final int code;

    SprtVerdict(int code) {
        this.code = code;
    }

    static SprtVerdict fromCode(int code) {
        for (SprtVerdict value : values()) {
            if (value.code == code) {
                return value;
            }
        }
        return INCONCLUSIVE;
    }
}