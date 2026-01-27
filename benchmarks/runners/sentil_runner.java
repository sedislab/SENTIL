import io.github.sedislab.sentil.Monitor;
import io.github.sedislab.sentil.OnlineMonitor;
import io.github.sedislab.sentil.Robustness;
import io.github.sedislab.sentil.Trace;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;

class SentilRunner {
    static final String FORMULA = "always[0, 100](eventually[0, 10](x > 5))";

    static double nowMs() {
        return System.nanoTime() / 1.0e6;
    }

    static final class Summary {
        final double mean;
        final double std;
        final double min;
        final double p50;
        final double p99;

        Summary(double[] samples) {
            double[] s = samples.clone();
            Arrays.sort(s);
            int n = s.length;
            double sum = 0;
            for (double x : s) {
                sum += x;
            }
            mean = sum / n;
            double acc = 0;
            for (double x : s) {
                acc += (x - mean) * (x - mean);
            }
            std = n > 1 ? Math.sqrt(acc / (n - 1)) : 0.0;
            min = s[0];
            p50 = s[Math.min(Math.max((int) Math.round((n - 1) * 0.50), 0), n - 1)];
            p99 = s[Math.min(Math.max((int) Math.round((n - 1) * 0.99), 0), n - 1)];
        }
    }

    static String cpuModel() {
        try {
            for (String line : Files.readAllLines(Path.of("/proc/cpuinfo"))) {
                if (line.startsWith("model name")) {
                    return line.substring(line.indexOf(':') + 1).trim();
                }
            }
        } catch (IOException ignored) {
        }
        return "unknown";
    }

    static long peakRssBytes() {
        try {
            for (String line : Files.readAllLines(Path.of("/proc/self/status"))) {
                if (line.startsWith("VmHWM:")) {
                    return Long.parseLong(line.split("\\s+")[1]) * 1024;
                }
            }
        } catch (IOException | NumberFormatException ignored) {
        }
        return -1;
    }

    static void emit(String benchmark, String question, long size, double robustness, Summary t,
            long runs) {
        long rss = peakRssBytes();
        String rssField = rss >= 0 ? Long.toString(rss) : "null";
        StringBuilder out = new StringBuilder();
        out.append("{\"tool\":\"sentil\",\"version\":\"0.3.0\",\"language\":\"java\",\"benchmark\":\"")
                .append(benchmark).append("\",");
        out.append(String.format("\"formula\":\"%s\",\"question\":\"%s\",\"size\":%d,\"robustness\":%.17g,",
                FORMULA, question, size, robustness));
        out.append(String.format(
                "\"timing\":{\"mean_ms\":%.17g,\"std_ms\":%.17g,\"min_ms\":%.17g,\"p50_ms\":%.17g,\"p99_ms\":%.17g},",
                t.mean, t.std, t.min, t.p50, t.p99));
        out.append(String.format("\"peak_rss_bytes\":%s,\"runs\":%d,\"hardware\":{\"cpu\":\"%s\",\"cores\":%d}}",
                rssField, runs, cpuModel(), Runtime.getRuntime().availableProcessors()));
        System.out.println(out);
    }

    static Trace oracleTrace(int n) throws Exception {
        double[] times = new double[n];
        double[] x = new double[n];
        for (int i = 0; i < n; i++) {
            times[i] = i;
            x[i] = 15.0 * Math.sin(i * 0.1);
        }
        Trace trace = Trace.create(times);
        trace.addSignal("x", x);
        return trace;
    }

    static void scalability() throws Exception {
        for (int n : new int[] {1000, 10000, 100000, 1000000, 10000000}) {
            int runs = n <= 100000 ? 30 : 5;
            try (Trace trace = oracleTrace(n); Monitor monitor = Monitor.parse(FORMULA)) {
                double fullRob = monitor.robustnessSignal(trace)[0];
                double[] samples = new double[runs];
                for (int r = 0; r < runs; r++) {
                    double t0 = nowMs();
                    monitor.robustnessSignal(trace);
                    samples[r] = nowMs() - t0;
                }
                emit("scalability/length", "full_signal", n, fullRob, new Summary(samples), runs);

                double monRob = monitor.robustness(trace);
                samples = new double[runs];
                for (int r = 0; r < runs; r++) {
                    double t0 = nowMs();
                    monitor.robustness(trace);
                    samples[r] = nowMs() - t0;
                }
                emit("scalability/length", "monitoring", n, monRob, new Summary(samples), runs);
            }
        }
    }

    static void streaming() throws Exception {
        try (OnlineMonitor monitor = OnlineMonitor.create(FORMULA)) {
            int index = (int) monitor.symbolIndex("x").orElseThrow(IllegalStateException::new);
            int n = 1000000;
            double[] latencies = new double[n];
            double[] packed = new double[1];
            double last = 0.0;
            for (int i = 0; i < n; i++) {
                packed[index] = 15.0 * Math.sin(i * 0.1);
                double t0 = nowMs();
                Robustness verdict = monitor.updatePacked(i, packed);
                latencies[i] = nowMs() - t0;
                last = verdict.lower();
            }
            emit("streaming", "monitoring", n, last, new Summary(latencies), n);
        }
    }

    public static void main(String[] args) throws Exception {
        String suite = args.length == 0 ? "" : args[0];
        if (suite.equals("scalability")) {
            scalability();
        } else if (suite.equals("streaming")) {
            streaming();
        } else {
            System.err.println("unknown suite `" + suite + "`; use `scalability` or `streaming`");
            System.exit(1);
        }
    }
}