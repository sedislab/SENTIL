package io.github.sedislab.sentil;

import java.util.Arrays;
import java.util.List;

/** A parsed PrSTL formula. The combinators consume their operands. */
public final class Formula extends NativeResource {
    Formula(long handle) {
        super(handle, NativeLib::formulaDestroy);
    }

    /** Parse a PrSTL formula such as {@code "always[0, 10](x > 0)"}. */
    public static Formula parse(String formula) throws SentilException {
        return new Formula(NativeLib.formulaParse(formula));
    }

    /** Rebuild a formula from JSON. */
    public static Formula fromJson(String json) throws SentilException {
        return new Formula(NativeLib.formulaFromJson(json));
    }

    /** The JSON form of this formula. */
    public String toJson() throws SentilException {
        return NativeLib.formulaToJson(handle());
    }

    /** The nesting depth, a predicate being 1. */
    public long depth() {
        return NativeLib.formulaDepth(handle());
    }

    /** Whether the formula uses any temporal operator. */
    public boolean hasTemporal() {
        return NativeLib.formulaHasTemporal(handle());
    }

    /** The variable names the formula reads, sorted and unique. */
    public List<String> variables() throws SentilException {
        return Arrays.asList(NativeLib.formulaVariables(handle()));
    }

    /** The robustness of the formula over a trace, reading the sample grid. */
    public double robustness(Trace trace) throws SentilException {
        return NativeLib.formulaRobustness(handle(), trace.handle());
    }

    /** The robustness over a trace, catching threshold crossings between samples. */
    public double robustnessDense(Trace trace) throws SentilException {
        return NativeLib.formulaRobustnessDense(handle(), trace.handle());
    }

    /** The robustness at every sample of the trace, reading the grid. */
    public double[] robustnessSignal(Trace trace) throws SentilException {
        return NativeLib.formulaRobustnessSignal(handle(), trace.handle());
    }

    /** The dense robustness at every sample of the trace. */
    public double[] robustnessDenseSignal(Trace trace) throws SentilException {
        return NativeLib.formulaRobustnessDenseSignal(handle(), trace.handle());
    }

    /** The time spans where the formula does not hold over the trace. */
    public List<Interval> violations(Trace trace) throws SentilException {
        return Arrays.asList(NativeLib.formulaViolations(handle(), trace.handle()));
    }

    /**
     * Estimate the satisfaction probability of this P-wrapped formula with the default
     * SMC settings.
     */
    public SmcResult check(Trace trace, LiftingRegistry lifting) throws SentilException {
        return check(trace, lifting, new SmcConfig());
    }

    /** Estimate the satisfaction probability with the given SMC settings. */
    public SmcResult check(Trace trace, LiftingRegistry lifting, SmcConfig config)
            throws SentilException {
        return NativeLib.formulaCheck(handle(), trace.handle(), lifting.handle(), config.samples,
                config.confidence, config.seed, config.method.code());
    }

    /** The check reported with the Clopper-Pearson interval. */
    public SmcResult checkConservative(Trace trace, LiftingRegistry lifting) throws SentilException {
        return checkConservative(trace, lifting, new SmcConfig());
    }

    /** The conservative check with the given SMC settings. */
    public SmcResult checkConservative(Trace trace, LiftingRegistry lifting, SmcConfig config)
            throws SentilException {
        return NativeLib.formulaCheckConservative(handle(), trace.handle(), lifting.handle(),
                config.samples, config.confidence, config.seed, config.method.code());
    }

    /** The check with the robustness distribution across the ensemble. */
    public SmcDistribution checkDistribution(Trace trace, LiftingRegistry lifting, SmcConfig config)
            throws SentilException {
        Object[] pair = NativeLib.formulaCheckDistribution(handle(), trace.handle(),
                lifting.handle(), config.samples, config.confidence, config.seed,
                config.method.code());
        return new SmcDistribution((SmcResult) pair[0], (RobustnessDistribution) pair[1]);
    }

    /** Decide this P-wrapped formula sequentially with Wald's SPRT. */
    public SprtResult checkSequential(Trace trace, LiftingRegistry lifting, SprtConfig config)
            throws SentilException {
        return NativeLib.formulaCheckSequential(handle(), trace.handle(), lifting.handle(),
                config.p0, config.p1, config.alpha, config.beta, config.maxSamples, config.seed);
    }

    /** Decide this P-wrapped formula with a Bayesian sequential test. */
    public BayesResult checkBayesian(Trace trace, LiftingRegistry lifting, BayesConfig config)
            throws SentilException {
        return NativeLib.formulaCheckBayesian(handle(), trace.handle(), lifting.handle(),
                config.threshold, config.bayesFactor, config.maxSamples, config.seed);
    }

    /**
     * Estimate this P-wrapped formula over a stochastic system by adaptive multilevel
     * splitting.
     */
    public RareEventResult checkRareEvent(StochasticSystem system) throws SentilException {
        return checkRareEvent(system, new RareEventConfig());
    }

    /** Estimate this P-wrapped formula by rare-event splitting with the given settings. */
    public RareEventResult checkRareEvent(StochasticSystem system, RareEventConfig config)
            throws SentilException {
        RareEventResult result = NativeLib.formulaCheckRareEvent(handle(), system.handle(),
                config.particles, config.margin, config.seed);
        system.rethrowCallbackError();
        return result;
    }

    /** The differentiable surrogate for robustness, with the default smoothing. */
    public double smoothRobustness(Trace trace) throws SentilException {
        return smoothRobustness(trace, new SmoothConfig());
    }

    /** The smooth robustness over a trace with the given smoothing settings. */
    public double smoothRobustness(Trace trace, SmoothConfig config) throws SentilException {
        return NativeLib.formulaSmoothRobustness(handle(), trace.handle(), config.temperature,
                config.kind.code());
    }

    /**
     * Search for a trajectory that violates this formula on the model by descending the
     * smooth robustness.
     */
    public Witness findCounterexample(SystemModel model, Bounds bounds, long maxIters,
            SmoothConfig smooth) throws SentilException {
        double temperature = smooth == null ? 0.0 : smooth.temperature;
        int kind = smooth == null ? 0 : smooth.kind.code();
        Witness witness = NativeLib.findCounterexample(handle(), model.handle(), bounds.handle(),
                maxIters, temperature, kind, smooth != null);
        return surface(model, witness);
    }

    /** Search for a violating trajectory globally with restarted CMA-ES on exact robustness. */
    public Witness falsify(SystemModel model, Bounds bounds, CmaConfig config, long restarts)
            throws SentilException {
        Witness witness = NativeLib.falsify(handle(), model.handle(), bounds.handle(),
                config.population, config.maxGenerations, config.initialStep, config.tolStep,
                config.seed, restarts);
        return surface(model, witness);
    }

    private static Witness surface(SystemModel model, Witness witness) {
        try {
            model.rethrowCallbackError();
        } catch (RuntimeException error) {
            witness.close();
            throw error;
        }
        return witness;
    }

    /**
     * The smooth robustness over a trace and its gradient, as [variable][sample] in
     * sorted variable order.
     */
    public SignalGradient smoothValueAndGradient(Trace trace, SmoothConfig config)
            throws SentilException {
        double[] packed = NativeLib.formulaSmoothValueAndGradient(handle(), trace.handle(),
                config.temperature, config.kind.code());
        int nVars = variables().size();
        int nSamples = packed.length == 0 || nVars == 0 ? 0 : (packed.length - 1) / nVars;
        double[][] gradient = new double[nVars][nSamples];
        for (int i = 0; i < nVars; i++) {
            System.arraycopy(packed, 1 + i * nSamples, gradient[i], 0, nSamples);
        }
        return new SignalGradient(packed[0], gradient);
    }

    /**
     * The smooth robustness of the trajectory the model rolls from initial under input,
     * with the gradient per input coordinate.
     */
    public InputGradient smoothGradient(SystemModel model, double[] initial, double[] input,
            SmoothConfig config) throws SentilException {
        double[] packed = NativeLib.formulaSmoothGradient(handle(), model.handle(), initial, input,
                config.temperature, config.kind.code());
        double[] gradient = new double[packed.length - 1];
        System.arraycopy(packed, 1, gradient, 0, gradient.length);
        return new InputGradient(packed[0], gradient);
    }

    /**
     * Estimate this P &gt;= p (always[0, b] psi) formula on the GPU by fixed-effort
     * multilevel splitting.
     */
    public GpuSplittingEstimate checkRareEventGpu(SimModel model, RareEventConfig config)
            throws SentilException {
        return NativeLib.formulaCheckRareEventGpu(handle(), model.handle(), config.particles,
                config.margin, config.seed);
    }

    /** The negation of this formula. */
    public Formula not() throws SentilException {
        return new Formula(NativeLib.formulaNot(consume()));
    }

    /** This formula and another both hold. */
    public Formula and(Formula other) throws SentilException {
        long left = handle();
        long right = other.handle();
        disown();
        other.disown();
        return new Formula(NativeLib.formulaAnd(left, right));
    }

    /** This formula or another holds. */
    public Formula or(Formula other) throws SentilException {
        long left = handle();
        long right = other.handle();
        disown();
        other.disown();
        return new Formula(NativeLib.formulaOr(left, right));
    }

    /** This formula implies a consequent. */
    public Formula implies(Formula consequent) throws SentilException {
        long left = handle();
        long right = consequent.handle();
        disown();
        consequent.disown();
        return new Formula(NativeLib.formulaImplies(left, right));
    }

    /** This formula holds at the next sample. */
    public Formula next() throws SentilException {
        return new Formula(NativeLib.formulaNext(consume()));
    }

    /** This formula holds at every time, with no upper bound on the window. */
    public Formula always() throws SentilException {
        return new Formula(NativeLib.formulaAlways(0.0, 0.0, false, consume()));
    }

    /** This formula holds throughout [lower, end of time). */
    public Formula always(double lower) throws SentilException {
        return new Formula(NativeLib.formulaAlways(lower, 0.0, false, consume()));
    }

    /** This formula holds throughout the window [lower, upper]. */
    public Formula always(double lower, double upper) throws SentilException {
        return new Formula(NativeLib.formulaAlways(lower, upper, true, consume()));
    }

    /** This formula holds at some future time. */
    public Formula eventually() throws SentilException {
        return new Formula(NativeLib.formulaEventually(0.0, 0.0, false, consume()));
    }

    /** This formula holds at some point in [lower, end of time). */
    public Formula eventually(double lower) throws SentilException {
        return new Formula(NativeLib.formulaEventually(lower, 0.0, false, consume()));
    }

    /** This formula holds at some point in [lower, upper]. */
    public Formula eventually(double lower, double upper) throws SentilException {
        return new Formula(NativeLib.formulaEventually(lower, upper, true, consume()));
    }

    /** This formula held at every past time. */
    public Formula historically() throws SentilException {
        return new Formula(NativeLib.formulaHistorically(0.0, 0.0, false, consume()));
    }

    /** This formula held throughout the past window [lower, end of time). */
    public Formula historically(double lower) throws SentilException {
        return new Formula(NativeLib.formulaHistorically(lower, 0.0, false, consume()));
    }

    /** This formula held throughout the past window [lower, upper]. */
    public Formula historically(double lower, double upper) throws SentilException {
        return new Formula(NativeLib.formulaHistorically(lower, upper, true, consume()));
    }

    /** This formula held at some past time. */
    public Formula once() throws SentilException {
        return new Formula(NativeLib.formulaOnce(0.0, 0.0, false, consume()));
    }

    /** This formula held at some past point in [lower, end of time). */
    public Formula once(double lower) throws SentilException {
        return new Formula(NativeLib.formulaOnce(lower, 0.0, false, consume()));
    }

    /** This formula held at some past point in [lower, upper]. */
    public Formula once(double lower, double upper) throws SentilException {
        return new Formula(NativeLib.formulaOnce(lower, upper, true, consume()));
    }

    /** This formula holds until another does, with no upper bound. */
    public Formula until(Formula right) throws SentilException {
        return until(right, 0.0, 0.0, false);
    }

    /** This formula holds until another does, within [lower, end of time). */
    public Formula until(Formula right, double lower) throws SentilException {
        return until(right, lower, 0.0, false);
    }

    /** This formula holds until another does, within [lower, upper]. */
    public Formula until(Formula right, double lower, double upper) throws SentilException {
        return until(right, lower, upper, true);
    }

    private Formula until(Formula right, double lower, double upper, boolean hasUpper)
            throws SentilException {
        long left = handle();
        long rightHandle = right.handle();
        disown();
        right.disown();
        return new Formula(NativeLib.formulaUntil(lower, upper, hasUpper, left, rightHandle));
    }

    /** This formula has held since another did, with no upper bound. */
    public Formula since(Formula right) throws SentilException {
        return since(right, 0.0, 0.0, false);
    }

    /** This formula has held since another did, within the past [lower, end of time). */
    public Formula since(Formula right, double lower) throws SentilException {
        return since(right, lower, 0.0, false);
    }

    /** This formula has held since another did, within the past [lower, upper]. */
    public Formula since(Formula right, double lower, double upper) throws SentilException {
        return since(right, lower, upper, true);
    }

    private Formula since(Formula right, double lower, double upper, boolean hasUpper)
            throws SentilException {
        long left = handle();
        long rightHandle = right.handle();
        disown();
        right.disown();
        return new Formula(NativeLib.formulaSince(lower, upper, hasUpper, left, rightHandle));
    }

    /**
     * Wrap this formula in a probabilistic operator, so
     * {@code probability(ProbabilityOp.GE, 0.9)} is P&gt;=0.9.
     */
    public Formula probability(ProbabilityOp op, double threshold) throws SentilException {
        return new Formula(NativeLib.formulaProbabilistic(op.code(), threshold, consume()));
    }
}