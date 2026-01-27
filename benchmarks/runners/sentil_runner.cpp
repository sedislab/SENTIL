// The SENTIL C++ runner. Emits one JSON record per measurement to standard output,
// matching the Rust runner's schema so the C++ binding is timed against the core on
// identical work. Run as `sentil_runner_cpp <scalability|streaming>`.
//
// It builds the x-only oracle trace the C runner builds, not the Rust runner's
// three-signal trace, so at the largest sizes it can read faster than the Rust
// number; that is a trace-composition artifact, not a real speedup. The full-signal
// path copies the core's result into a std::vector, the one cost the C++ idiom adds
// over the raw C call; the streaming path crosses the boundary once per sample with
// no allocation.
#include <sentil/sentil.hpp>

#include <unistd.h>

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <fstream>
#include <string>
#include <vector>

static double now_ms() {
    auto t = std::chrono::steady_clock::now().time_since_epoch();
    return std::chrono::duration<double, std::milli>(t).count();
}

struct Timing {
    double mean, std, min, p50, p99;
};

static double percentile(const std::vector<double>& sorted, double q) {
    return sorted[static_cast<std::size_t>(std::llround((sorted.size() - 1) * q))];
}

static Timing summarize(std::vector<double> s) {
    std::sort(s.begin(), s.end());
    double sum = 0.0;
    for (double v : s) {
        sum += v;
    }
    double mean = sum / s.size();
    double var = 0.0;
    if (s.size() > 1) {
        for (double v : s) {
            var += (v - mean) * (v - mean);
        }
        var /= (s.size() - 1);
    }
    return Timing{mean, std::sqrt(var), s.front(), percentile(s, 0.50), percentile(s, 0.99)};
}

static std::string cpu_model() {
    std::ifstream f("/proc/cpuinfo");
    std::string line;
    while (std::getline(f, line)) {
        if (line.rfind("model name", 0) == 0) {
            std::size_t colon = line.find(':');
            if (colon != std::string::npos) {
                std::size_t start = line.find_first_not_of(' ', colon + 1);
                return start == std::string::npos ? "unknown" : line.substr(start);
            }
        }
    }
    return "unknown";
}

static long peak_rss_bytes() {
    std::ifstream f("/proc/self/status");
    std::string line;
    while (std::getline(f, line)) {
        if (line.rfind("VmHWM:", 0) == 0) {
            return std::stol(line.substr(6)) * 1024;
        }
    }
    return -1;
}

static void emit(const char* benchmark, const char* formula, const char* question,
                 std::uint64_t size, double robustness, const Timing& t, std::uint64_t runs) {
    long rss = peak_rss_bytes();
    std::printf("{\"tool\":\"sentil\",\"version\":\"0.3.0\",\"language\":\"cpp\",\"benchmark\":\"%s\","
                "\"formula\":\"%s\",\"question\":\"%s\",\"size\":%llu,\"robustness\":%.17g,",
                benchmark, formula, question, static_cast<unsigned long long>(size), robustness);
    std::printf("\"timing\":{\"mean_ms\":%.17g,\"std_ms\":%.17g,\"min_ms\":%.17g,\"p50_ms\":%.17g,"
                "\"p99_ms\":%.17g},",
                t.mean, t.std, t.min, t.p50, t.p99);
    if (rss >= 0) {
        std::printf("\"peak_rss_bytes\":%ld,", rss);
    } else {
        std::printf("\"peak_rss_bytes\":null,");
    }
    std::printf("\"runs\":%llu,\"hardware\":{\"cpu\":\"%s\",\"cores\":%ld}}\n",
                static_cast<unsigned long long>(runs), cpu_model().c_str(),
                sysconf(_SC_NPROCESSORS_ONLN));
}

static sentil::Trace oracle_trace(std::size_t n) {
    std::vector<double> times(n);
    std::vector<double> x(n);
    for (std::size_t i = 0; i < n; ++i) {
        times[i] = static_cast<double>(i);
        x[i] = 15.0 * std::sin(static_cast<double>(i) * 0.1);
    }
    return sentil::Trace(times, "x", x);
}

static const char* FORMULA = "always[0, 100](eventually[0, 10](x > 5))";

static void scalability() {
    std::uint64_t sizes[] = {1000, 10000, 100000, 1000000, 10000000};
    for (std::uint64_t n : sizes) {
        std::uint64_t runs = n <= 100000 ? 30 : 5;
        sentil::Trace trace = oracle_trace(static_cast<std::size_t>(n));
        sentil::Monitor monitor(FORMULA);

        double full_rob = monitor.robustness_signal(trace).front();
        std::vector<double> samples(runs);
        for (std::uint64_t r = 0; r < runs; ++r) {
            double t0 = now_ms();
            std::vector<double> out = monitor.robustness_signal(trace);
            samples[r] = now_ms() - t0;
        }
        emit("scalability/length", FORMULA, "full_signal", n, full_rob, summarize(samples), runs);

        double mon_rob = monitor.robustness(trace);
        for (std::uint64_t r = 0; r < runs; ++r) {
            double t0 = now_ms();
            double out = monitor.robustness(trace);
            samples[r] = now_ms() - t0;
            (void)out;
        }
        emit("scalability/length", FORMULA, "monitoring", n, mon_rob, summarize(samples), runs);
    }
}

static void streaming() {
    sentil::OnlineMonitor monitor(FORMULA);
    std::size_t idx = monitor.symbol_index("x").value();
    std::size_t n = 1000000;
    std::vector<double> latencies(n);
    std::vector<double> packed(1, 0.0);
    double last = 0.0;
    for (std::size_t i = 0; i < n; ++i) {
        packed[idx] = 15.0 * std::sin(static_cast<double>(i) * 0.1);
        double t0 = now_ms();
        sentil::Robustness out = monitor.update_packed(static_cast<double>(i), packed);
        latencies[i] = now_ms() - t0;
        last = out.lower;
    }
    emit("streaming", FORMULA, "monitoring", n, last, summarize(latencies), n);
}

int main(int argc, char** argv) {
    std::string suite = argc > 1 ? argv[1] : "";
    if (suite == "scalability") {
        scalability();
    } else if (suite == "streaming") {
        streaming();
    } else {
        std::fprintf(stderr, "unknown suite `%s`; use `scalability` or `streaming`\n",
                     suite.c_str());
        return 1;
    }
    return 0;
}