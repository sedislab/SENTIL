package io.github.sedislab.sentil;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.Locale;

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
}