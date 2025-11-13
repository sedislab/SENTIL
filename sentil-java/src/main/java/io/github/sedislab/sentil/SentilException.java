package io.github.sedislab.sentil;

/** The base of every error SENTIL raises. */
public class SentilException extends Exception {
    private static final long serialVersionUID = 1L;

    private final int code;
    private final transient ErrorCode errorCode;

    /** Build an exception with the engine's message and its C ABI status code. */
    public SentilException(String message, int code) {
        super(message);
        this.code = code;
        this.errorCode = ErrorCode.fromCode(code);
    }

    /** The C ABI status integer behind this error. */
    public int code() {
        return code;
    }

    /** The status code as an {@link ErrorCode}. */
    public ErrorCode errorCode() {
        return errorCode;
    }
}