package io.github.sedislab.sentil;

/** A formula failed to parse. */
public class ParseException extends SentilException {
    private static final long serialVersionUID = 1L;

    public ParseException(String message, int code) {
        super(message, code);
    }
}