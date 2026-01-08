package io.github.sedislab.sentil;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.Locale;
import java.util.Map;

/** The loader for the native library and the raw native methods behind the typed classes. */
final class NativeLib {
    private NativeLib() {
    }

    static {
        load();
    }

    private static void load() {
        String dir = "/native/" + classifier() + "/";
        try {
            Path tmp = Files.createTempDirectory("sentil-native");
            tmp.toFile().deleteOnExit();
            Path core = extract(dir, System.mapLibraryName("sentil"), tmp);
            Path shim = extract(dir, System.mapLibraryName("sentil_jni"), tmp);
            System.load(core.toAbsolutePath().toString());
            System.load(shim.toAbsolutePath().toString());
        } catch (IOException e) {
            throw new UncheckedIOException("could not load the SENTIL native library", e);
        }
    }

    private static Path extract(String dir, String name, Path into) throws IOException {
        try (InputStream in = NativeLib.class.getResourceAsStream(dir + name)) {
            if (in == null) {
                throw new IOException("no native library at " + dir + name
                        + "; this jar carries no build for this platform");
            }
            Path out = into.resolve(name);
            Files.copy(in, out, StandardCopyOption.REPLACE_EXISTING);
            out.toFile().deleteOnExit();
            return out;
        }
    }

    private static String classifier() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        String arch = System.getProperty("os.arch", "").toLowerCase(Locale.ROOT);
        String osName;
        if (os.contains("win")) {
            osName = "windows";
        } else if (os.contains("mac") || os.contains("darwin")) {
            osName = "darwin";
        } else {
            osName = "linux";
        }
        String archName;
        if (arch.equals("amd64") || arch.equals("x86_64")) {
            archName = "x86_64";
        } else if (arch.equals("aarch64") || arch.equals("arm64")) {
            archName = "aarch64";
        } else {
            archName = arch;
        }
        return osName + "-" + archName;
    }

    static native int[] version();

    static native long formulaParse(String formula) throws SentilException;

    static native void formulaDestroy(long handle);

    static native String formulaToJson(long handle) throws SentilException;

    static native long formulaFromJson(String json) throws SentilException;

    static native long formulaDepth(long handle);

    static native boolean formulaHasTemporal(long handle);

    static native String[] formulaVariables(long handle) throws SentilException;

    static native long traceCreate(double[] times) throws SentilException;

    static native long traceFromSignal(double[] times, String name, double[] values)
            throws SentilException;

    static native long traceIndexed(long length) throws SentilException;

    static native void traceAddSignal(long handle, String name, double[] values)
            throws SentilException;

    static native long traceLen(long handle);

    static native boolean traceIsEmpty(long handle);

    static native double[] traceTimes(long handle);

    static native String[] traceVariables(long handle) throws SentilException;

    static native double[] traceSignal(long handle, String name);

    static native void traceDestroy(long handle);

    static native double formulaRobustness(long formula, long trace) throws SentilException;

    static native double formulaRobustnessDense(long formula, long trace) throws SentilException;

    static native double[] formulaRobustnessSignal(long formula, long trace) throws SentilException;

    static native double[] formulaRobustnessDenseSignal(long formula, long trace)
            throws SentilException;

    static native long exprVariable(String name) throws SentilException;

    static native long exprLiteral(double value) throws SentilException;

    static native long exprBinary(int op, long left, long right) throws SentilException;

    static native long exprCall(String name, long[] args) throws SentilException;

    static native void exprDestroy(long handle);

    static native long formulaPredicate(long left, int op, long right) throws SentilException;

    static native long formulaNot(long child) throws SentilException;

    static native long formulaAnd(long left, long right) throws SentilException;

    static native long formulaOr(long left, long right) throws SentilException;

    static native long formulaImplies(long left, long right) throws SentilException;

    static native long formulaNext(long child) throws SentilException;

    static native long formulaAlways(double lower, double upper, boolean hasUpper, long child)
            throws SentilException;

    static native long formulaEventually(double lower, double upper, boolean hasUpper, long child)
            throws SentilException;

    static native long formulaHistorically(double lower, double upper, boolean hasUpper, long child)
            throws SentilException;

    static native long formulaOnce(double lower, double upper, boolean hasUpper, long child)
            throws SentilException;

    static native long formulaUntil(double lower, double upper, boolean hasUpper, long left,
            long right) throws SentilException;

    static native long formulaSince(double lower, double upper, boolean hasUpper, long left,
            long right) throws SentilException;

    static native long formulaProbabilistic(int op, double threshold, long child)
            throws SentilException;

    static native long traceResample(long handle, double[] times, int interp)
            throws SentilException;

    static native long tracePrepare(long handle, int interp) throws SentilException;

    static native long preparedTraceResample(long prepared, double[] times) throws SentilException;

    static native void preparedTraceDestroy(long handle);

    static native long traceFromCsv(String text) throws SentilException;

    static native long traceFromTsv(String text) throws SentilException;

    static native long traceFromPath(String path) throws SentilException;

    static native long ringBufferCreate(long capacity) throws SentilException;

    static native Sample ringBufferPush(long handle, double time, double value)
            throws SentilException;

    static native void ringBufferClear(long handle);

    static native long ringBufferLen(long handle);

    static native long ringBufferCapacity(long handle);

    static native boolean ringBufferIsEmpty(long handle);

    static native boolean ringBufferIsFull(long handle);

    static native Sample ringBufferFront(long handle);

    static native Sample ringBufferBack(long handle);

    static native Sample ringBufferGet(long handle, long index);

    static native Sample ringBufferPopFront(long handle);

    static native Sample ringBufferPopBack(long handle);

    static native Sample ringBufferClosestToTime(long handle, double time);

    static native double[] ringBufferMean(long handle);

    static native double[] ringBufferVariance(long handle);

    static native double[] ringBufferStdDev(long handle);

    static native double[] ringBufferMin(long handle);

    static native double[] ringBufferMax(long handle);

    static native void ringBufferRecomputeStatistics(long handle);

    static native double[] ringBufferAtTime(long handle, double time);

    static native double[] ringBufferTimeRange(long handle);

    static native Sample[] ringBufferBetween(long handle, double start, double end)
            throws SentilException;

    static native void ringBufferDestroy(long handle);

    static native long configCreate() throws SentilException;

    static native void configSetTime(long handle, int mode) throws SentilException;

    static native int configTimeMode(long handle);

    static native void configDestroy(long handle);

    static native Interval[] formulaViolations(long formula, long trace) throws SentilException;

    static native long monitorCreate(long formula, long config) throws SentilException;

    static native long monitorParse(String formula, long config) throws SentilException;

    static native long monitorFormula(long handle) throws SentilException;

    static native long monitorConfig(long handle) throws SentilException;

    static native double monitorRobustness(long handle, long trace) throws SentilException;

    static native double[] monitorRobustnessSignal(long handle, long trace) throws SentilException;

    static native Interval[] monitorViolations(long handle, long trace) throws SentilException;

    static native long[] monitorSymbolIndex(long handle, String name) throws SentilException;

    static native Robustness monitorUpdate(long handle, double time, String[] names, double[] values)
            throws SentilException;

    static native Robustness monitorUpdatePacked(long handle, double time, double[] values)
            throws SentilException;

    static native void monitorReset(long handle);

    static native double monitorLastProbability(long handle);

    static native void monitorDestroy(long handle);

    static native long streamMonitorCreate(String formula) throws SentilException;

    static native long streamMonitorFromFormula(long formula) throws SentilException;

    static native long streamMonitorVariableCount(long handle);

    static native long[] streamMonitorSymbolIndex(long handle, String name) throws SentilException;

    static native Robustness streamMonitorUpdate(long handle, double time, String[] names,
            double[] values) throws SentilException;

    static native Robustness streamMonitorUpdatePacked(long handle, double time, double[] values)
            throws SentilException;

    static native Robustness[] streamMonitorRun(long handle, long trace) throws SentilException;

    static native void streamMonitorReset(long handle);

    static native double streamMonitorLastProbability(long handle);

    static native void streamMonitorDestroy(long handle);

    static native long multiMonitorCreate() throws SentilException;

    static native void multiMonitorAdd(long handle, String id, String formula)
            throws SentilException;

    static native void multiMonitorAddFormula(long handle, String id, long formula)
            throws SentilException;

    static native boolean multiMonitorRemove(long handle, String id);

    static native void multiMonitorReset(long handle);

    static native double multiMonitorProbability(long handle, String id);

    static native long multiMonitorLen(long handle);

    static native boolean multiMonitorIsEmpty(long handle);

    static native String[] multiMonitorIds(long handle) throws SentilException;

    static native Map<String, Robustness> multiMonitorUpdate(long handle, double time, String[] names,
            double[] values) throws SentilException;

    static native void multiMonitorDestroy(long handle);

    static native long formulaBankCreate() throws SentilException;

    static native void formulaBankAdd(long handle, String id, String formula)
            throws SentilException;

    static native void formulaBankAddFormula(long handle, String id, long formula)
            throws SentilException;

    static native String[] formulaBankIds(long handle) throws SentilException;

    static native long formulaBankLen(long handle);

    static native boolean formulaBankIsEmpty(long handle);

    static native Map<String, Double> formulaBankRobustness(long handle, long trace)
            throws SentilException;

    static native Map<String, Double> formulaBankRobustnessDense(long handle, long trace)
            throws SentilException;

    static native void formulaBankDestroy(long handle);

    static native ConfidenceInterval wilsonInterval(long successes, long trials, double level);

    static native ConfidenceInterval clopperPearson(long successes, long trials, double level);

    static native ConfidenceInterval jeffreysInterval(long successes, long trials, double level);

    static native ConfidenceInterval agrestiCoull(long successes, long trials, double level);

    static native ConfidenceInterval intervalByMethod(int method, long successes, long trials,
            double level);

    static native double zScore(double level);

    static native long chernoffHoeffdingSamples(double epsilon, double delta) throws SentilException;

    static native long wilsonSamples(double epsilon, double level) throws SentilException;

    static native long noiseDirac(double value) throws SentilException;

    static native long noiseGaussian(double mean, double stdDev) throws SentilException;

    static native long noiseUniform(double low, double high) throws SentilException;

    static native long noiseLogNormal(double mu, double sigma) throws SentilException;

    static native long noiseExponential(double rate) throws SentilException;

    static native long noiseGamma(double shape, double scale) throws SentilException;

    static native long noiseBeta(double alpha, double beta) throws SentilException;

    static native long noiseWeibull(double shape, double scale) throws SentilException;

    static native long noiseRayleigh(double scale) throws SentilException;

    static native long noiseGumbel(double location, double scale) throws SentilException;

    static native long noiseCauchy(double location, double scale) throws SentilException;

    static native long noiseStudentT(double df, double location, double scale) throws SentilException;

    static native long noiseTruncatedNormal(double mean, double stdDev, double lower, double upper)
            throws SentilException;

    static native long noisePoisson(double rate) throws SentilException;

    static native long noiseBinomial(long n, double p) throws SentilException;

    static native long noiseBootstrap(double[] residuals) throws SentilException;

    static native double[] noiseMean(long handle);

    static native double[] noiseVariance(long handle);

    static native String noiseToJson(long handle) throws SentilException;

    static native long noiseFromJson(String json) throws SentilException;

    static native long noiseFromFile(String path) throws SentilException;

    static native void noiseDestroy(long handle);

    static native long noiseFitGaussian(double[] samples) throws SentilException;

    static native long noiseFitBootstrap(double[] samples) throws SentilException;

    static native long noiseFitBootstrapReservoir(double[] samples, long maxSamples)
            throws SentilException;

    static native long noiseFitGaussianMixture(double[] samples, long components, long maxIters)
            throws SentilException;

    static native double[] noiseResiduals(double[] groundTruth, double[] sensor, int interaction)
            throws SentilException;

    static native long noiseMixture(double[] weights, long[] models) throws SentilException;

    static native long liftingCreate() throws SentilException;

    static native void liftingRegister(long handle, String variable, long model, int interaction)
            throws SentilException;

    static native String[] liftingVariables(long handle) throws SentilException;

    static native boolean liftingIsEmpty(long handle);

    static native long liftingLift(long handle, long trace, long seed) throws SentilException;

    static native void liftingDestroy(long handle);

    static native SmcResult formulaCheck(long formula, long trace, long lifting, long samples,
            double confidence, long seed, int method) throws SentilException;

    static native SmcResult formulaCheckConservative(long formula, long trace, long lifting,
            long samples, double confidence, long seed, int method) throws SentilException;

    static native Object[] formulaCheckDistribution(long formula, long trace, long lifting,
            long samples, double confidence, long seed, int method) throws SentilException;

    static native SmcResult monitorCheck(long monitor, long trace, long lifting)
            throws SentilException;

    static native SprtResult formulaCheckSequential(long formula, long trace, long lifting,
            double p0, double p1, double alpha, double beta, long maxSamples, long seed)
            throws SentilException;

    static native SprtResult monitorCheckSequential(long monitor, long trace, long lifting,
            double p0, double p1, double alpha, double beta, long maxSamples, long seed)
            throws SentilException;

    static native BayesResult formulaCheckBayesian(long formula, long trace, long lifting,
            double threshold, double bayesFactor, long maxSamples, long seed) throws SentilException;

    static native long streamMonitorWithLifting(long formula, long lifting, long samples,
            double confidence, long seed, int method) throws SentilException;

    static native void multiMonitorAddProbabilistic(long monitor, String id, long formula,
            long lifting, long samples, double confidence, long seed, int method)
            throws SentilException;

    static native SprtResult sequentialTest(double p0, double p1, double alpha, double beta,
            long maxSamples, long seed, java.util.function.BooleanSupplier draw)
            throws SentilException;

    static native BayesResult bayesSequentialTest(double threshold, double bayesFactor,
            long maxSamples, long seed, java.util.function.BooleanSupplier draw)
            throws SentilException;

    static native long simExprPrev(long variable) throws SentilException;

    static native long simExprTime() throws SentilException;

    static native long simExprConst(double value) throws SentilException;

    static native long simExprNoise(long source) throws SentilException;

    static native long simExprAdd(long left, long right) throws SentilException;

    static native long simExprSub(long left, long right) throws SentilException;

    static native long simExprMul(long left, long right) throws SentilException;

    static native long simExprDiv(long left, long right) throws SentilException;

    static native long simExprCall(String name, long[] args) throws SentilException;

    static native void simExprDestroy(long handle);

    static native long simModelCreate(String[] variables, double dt, long horizon, long[] init,
            long[] advance, long[] noise) throws SentilException;

    static native long simModelSimulate(long handle, long seed) throws SentilException;

    static native String[] simModelVariables(long handle) throws SentilException;

    static native double simModelDt(long handle);

    static native long simModelHorizon(long handle);

    static native long simModelToStochasticSystem(long handle) throws SentilException;

    static native void simModelDestroy(long handle);

    static native long stochasticSystemSimulate(long handle, long seed) throws SentilException;

    static native String[] stochasticSystemVariables(long handle) throws SentilException;

    static native double stochasticSystemDt(long handle);

    static native long stochasticSystemHorizon(long handle);

    static native void stochasticSystemDestroy(long handle);

    static native RareEventResult formulaCheckRareEvent(long formula, long system, long particles,
            double margin, long seed) throws SentilException;

    static native RareEventResult monitorCheckRare(long monitor, long system) throws SentilException;

    static native double softMin(double[] values, double temperature);

    static native double softMax(double[] values, double temperature);

    static native double formulaSmoothRobustness(long formula, long trace, double temperature,
            int kind) throws SentilException;

    static native long boundsCreate(double[] lower, double[] upper) throws SentilException;

    static native long boundsUnbounded(long dimension) throws SentilException;

    static native long boundsDimension(long handle);

    static native double[] boundsLower(long handle);

    static native double[] boundsUpper(long handle);

    static native double[] boundsClamp(long handle, double[] point);

    static native void boundsDestroy(long handle);

    static native long linearModelCreate(double[] a, long n, double[] b, long bCols, double[] x0,
            String[] variables, double dt, long horizon) throws SentilException;

    static native long systemModelInputDimension(long handle);

    static native void systemModelDestroy(long handle);

    static native SynthesisResult synthesize(long model, long spec, long bounds,
            double smoothTemperature, int smoothKind, boolean hasSmooth, long maxIters, int backend,
            long population) throws SentilException;

    static native double[] solveQp(double[] p, long n, double[] q, double[] g, long m, double[] h,
            long maxIters) throws SentilException;

    static native double[] solveSpd(double[] matrix, long n, double[] rhs) throws SentilException;

    static native double[] symmetricEigen(double[] matrix, long n) throws SentilException;

    static native long safetyFilterCreate(long bounds) throws SentilException;

    static native double[] safetyFilterFilter(long filter, double[] nominal, double[] barrierA,
            double[] barrierB, long m) throws SentilException;

    static native void safetyFilterDestroy(long handle);

    static native long chanceConstraintCreate(long spec, double probability, double confidence,
            double tightening) throws SentilException;

    static native ChanceReport chanceConstraintValidate(long constraint, long system, long samples,
            long seed) throws SentilException;

    static native void chanceConstraintDestroy(long handle);

    static native long controllerCreate(long model, long spec, long inputWidth, long budgetNs,
            long bounds, double smoothTemperature, int smoothKind, boolean hasSmooth)
            throws SentilException;

    static native double[] controllerControl(long controller, double[] state, long inputWidth)
            throws SentilException;

    static native void controllerDestroy(long handle);

    static native Witness findCounterexample(long formula, long model, long bounds, long maxIters,
            double smoothTemperature, int smoothKind, boolean hasSmooth) throws SentilException;

    static native Witness falsify(long formula, long model, long bounds, long population,
            long maxGenerations, double initialStep, double tolStep, long seed, long restarts)
            throws SentilException;

    static native double[] formulaSmoothValueAndGradient(long formula, long trace,
            double temperature, int kind) throws SentilException;

    static native double[] formulaSmoothGradient(long formula, long model, double[] initial,
            double[] input, double temperature, int kind) throws SentilException;

    static native double[] maximize(GradientObjective objective, double[] start, long bounds,
            long maxIters) throws SentilException;

    static native double[] cmaEs(java.util.function.ToDoubleFunction<double[]> objective,
            double[] start, long bounds, long population, long maxGenerations, double initialStep,
            double tolStep, long seed) throws SentilException;

    static native double[] cmaEsBatched(BatchObjective objective, double[] start, long bounds,
            long population, long maxGenerations, double initialStep, double tolStep, long seed)
            throws SentilException;

    static native long[] stochasticSystemCreateCustom(String[] variables, double dt, long horizon,
            SystemInit init, SystemStep step) throws SentilException;

    static native void freeSystemBox(long boxPointer);

    static native void rethrowSystemError(long boxPointer);

    static native double mineTightestParameter(ParameterFormula make, long[] traces, double lower,
            double upper) throws SentilException;

    static native long[] systemModelCreateCustom(String[] variables, double dt, long horizon,
            double[] initialState, long inputDimension, Rollout rollout) throws SentilException;

    static native void freeModelBox(long boxPointer);

    static native void rethrowModelError(long boxPointer);

    static native RareEventEstimate adaptiveMultilevelSplitting(AmsInterface simulator,
            long particles, double targetScore, long maxSteps, long seed) throws SentilException;

    static native String[] specRegistryAvailable() throws SentilException;

    static native long specBuilderCreate(String name) throws SentilException;

    static native long specBuilderFromFile(String path) throws SentilException;

    static native long specBuilderWithVariant(long handle, String variant) throws SentilException;

    static native long specBuilderWithParam(long handle, String name, double value)
            throws SentilException;

    static native String[] specBuilderAvailableVariants(long handle) throws SentilException;

    static native String specBuilderBuildDeterministic(long handle) throws SentilException;

    static native String specBuilderBuildProbabilistic(long handle) throws SentilException;

    static native long specBuilderBuildFormula(long handle) throws SentilException;

    static native long specBuilderBuildProbabilisticFormula(long handle) throws SentilException;

    static native long specBuilderBuildLiftingRegistry(long handle) throws SentilException;

    static native String specBuilderParametersJson(long handle) throws SentilException;

    static native long specBuilderIntoMonitor(long handle) throws SentilException;

    static native SpecSmcSettings specBuilderSmcSettings(long handle);

    static native SpecSprtSettings specBuilderSprtSettings(long handle);

    static native SpecAmsSettings specBuilderAmsSettings(long handle);

    static native void specBuilderDestroy(long handle);

    static native boolean gpuIsAvailable();

    static native GpuSplittingEstimate formulaCheckRareEventGpu(long formula, long model,
            long particles, double margin, long seed) throws SentilException;
}