package io.github.sedislab.sentil;

/** A status code returned by the SENTIL engine across the C ABI. */
public enum ErrorCode {
    /** No error. */
    OK(0),
    /** A required handle or argument was null. */
    NULL_POINTER(1),
    /** A string argument was not valid UTF-8. */
    UTF8(2),
    /** A formula failed to parse. */
    PARSE(3),
    /** A formula refers to a variable the trace does not carry. */
    UNKNOWN_VARIABLE(4),
    /** Robustness or a related quantity could not be evaluated. */
    EVALUATION(5),
    /** A trace was malformed. */
    TRACE(6),
    /** A probabilistic operation met a formula with no probabilistic operator. */
    NOT_PROBABILISTIC(7),
    /** A noise model had invalid parameters. */
    INVALID_NOISE_MODEL(8),
    /** A configuration value was out of range. */
    INVALID_CONFIG(9),
    /** A distribution fit did not converge or had too little data. */
    FIT(10),
    /** Reading a trace from a file or text failed. */
    INGEST(11),
    /** Rare-event splitting could not proceed. */
    SPLITTING(12),
    /** The requested construct is not supported on this path. */
    UNSUPPORTED(13),
    /** A formula or model could not be lowered to a GPU kernel. */
    TRANSPILATION(14),
    /** No usable GPU device was present, or a device call failed. */
    GPU(15),
    /** JSON serialization or deserialization failed. */
    JSON(16),
    /** An internal invariant was violated. */
    PANIC(17),
    /** A code outside the known set. */
    UNKNOWN(-1);

    private final int code;

    ErrorCode(int code) {
        this.code = code;
    }

    /** The integer value the C ABI uses for this status. */
    public int code() {
        return code;
    }

    /** The constant for a C ABI status integer, or {@link #UNKNOWN} if unrecognized. */
    public static ErrorCode fromCode(int code) {
        for (ErrorCode value : values()) {
            if (value.code == code) {
                return value;
            }
        }
        return UNKNOWN;
    }
}