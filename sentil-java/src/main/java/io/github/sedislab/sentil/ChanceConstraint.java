package io.github.sedislab.sentil;

/** A requirement that a spec holds with at least a target probability. */
public final class ChanceConstraint extends NativeResource {
    /** The constraint that spec, which is consumed, holds with probability at least probability. */
    public ChanceConstraint(Formula spec, double probability) throws SentilException {
        this(spec, probability, 0.0, 0.0);
    }

    /**
     * The constraint that spec, which is consumed, holds with probability at least
     * probability, at a confidence level and with a conservative tightening.
     */
    public ChanceConstraint(Formula spec, double probability, double confidence, double tightening)
            throws SentilException {
        super(NativeLib.chanceConstraintCreate(spec.consume(), probability, confidence, tightening),
                NativeLib::chanceConstraintDestroy);
    }

    /** Validate the constraint over the system with default sampling. */
    public ChanceReport validate(StochasticSystem system) throws SentilException {
        return validate(system, 1000, 42);
    }

    /** Validate the constraint over a given number of sampled trajectories. */
    public ChanceReport validate(StochasticSystem system, long samples, long seed)
            throws SentilException {
        ChanceReport report =
                NativeLib.chanceConstraintValidate(handle(), system.handle(), samples, seed);
        system.rethrowCallbackError();
        return report;
    }
}