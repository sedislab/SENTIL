package io.github.sedislab.sentil;

/** An evaluation, data, fit, numeric, or device error. */
public class EvaluationException extends SentilException {
    private static final long serialVersionUID = 1L;

    public EvaluationException(String message, int code) {
        super(message, code);
    }
}