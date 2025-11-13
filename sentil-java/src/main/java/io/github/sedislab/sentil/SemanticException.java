package io.github.sedislab.sentil;

/** A well-formed formula means something the engine cannot act on. */
public class SemanticException extends SentilException {
    private static final long serialVersionUID = 1L;

    public SemanticException(String message, int code) {
        super(message, code);
    }
}