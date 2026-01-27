/* The SENTIL C runner. Emits one JSON record per measurement to standard output,
   matching the Rust runner's schema so the C path is timed against the core on
   identical work. Run as `sentil_runner <scalability|streaming>`. */

#define _POSIX_C_SOURCE 200809L

#include "sentil.h"
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#define VERSION "0.3.0"

static double now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1e3 + (double)ts.tv_nsec * 1e-6;
}

static int cmp_double(const void *a, const void *b) {
    double x = *(const double *)a, y = *(const double *)b;
    return (x > y) - (x < y);
}

typedef struct {
    double mean, std, min, p50, p99;
} timing_t;

static double percentile(const double *sorted, size_t n, double q) {
    return sorted[(size_t)llround((double)(n - 1) * q)];
}

static timing_t summarize(double *s, size_t n) {
    qsort(s, n, sizeof(double), cmp_double);
    double sum = 0.0;
    for (size_t i = 0; i < n; i++) sum += s[i];
    double mean = sum / (double)n, var = 0.0;
    if (n > 1) {
        for (size_t i = 0; i < n; i++) var += (s[i] - mean) * (s[i] - mean);
        var /= (double)(n - 1);
    }
    timing_t t = {mean, sqrt(var), s[0], percentile(s, n, 0.50), percentile(s, n, 0.99)};
    return t;
}

static void cpu_model(char *out, size_t cap) {
    strncpy(out, "unknown", cap);
    out[cap - 1] = 0;
    FILE *f = fopen("/proc/cpuinfo", "r");
    if (!f) return;
    char line[512];
    while (fgets(line, sizeof line, f)) {
        if (strncmp(line, "model name", 10) != 0) continue;
        char *colon = strchr(line, ':');
        if (colon) {
            char *name = colon + 1;
            while (*name == ' ') name++;
            size_t len = strcspn(name, "\n");
            if (len >= cap) len = cap - 1;
            memcpy(out, name, len);
            out[len] = 0;
        }
        break;
    }
    fclose(f);
}

static long peak_rss_bytes(void) {
    FILE *f = fopen("/proc/self/status", "r");
    if (!f) return -1;
    char line[256];
    long kib = -1;
    while (fgets(line, sizeof line, f)) {
        if (strncmp(line, "VmHWM:", 6) == 0) {
            sscanf(line + 6, "%ld", &kib);
            break;
        }
    }
    fclose(f);
    return kib < 0 ? -1 : kib * 1024;
}

static void emit(const char *benchmark, const char *formula, const char *question, uint64_t size,
                 double robustness, timing_t t, uint64_t runs) {
    char cpu[256];
    cpu_model(cpu, sizeof cpu);
    long rss = peak_rss_bytes();
    printf("{\"tool\":\"sentil\",\"version\":\"%s\",\"language\":\"c\",\"benchmark\":\"%s\","
           "\"formula\":\"%s\",\"question\":\"%s\",\"size\":%llu,\"robustness\":%.17g,",
           VERSION, benchmark, formula, question, (unsigned long long)size, robustness);
    printf("\"timing\":{\"mean_ms\":%.17g,\"std_ms\":%.17g,\"min_ms\":%.17g,\"p50_ms\":%.17g,"
           "\"p99_ms\":%.17g},",
           t.mean, t.std, t.min, t.p50, t.p99);
    if (rss >= 0)
        printf("\"peak_rss_bytes\":%ld,", rss);
    else
        printf("\"peak_rss_bytes\":null,");
    printf("\"runs\":%llu,\"hardware\":{\"cpu\":\"%s\",\"cores\":%ld}}\n", (unsigned long long)runs,
           cpu, sysconf(_SC_NPROCESSORS_ONLN));
}

static sentil_trace_t *oracle_trace(size_t n) {
    double *times = malloc(n * sizeof(double));
    double *x = malloc(n * sizeof(double));
    for (size_t i = 0; i < n; i++) {
        times[i] = (double)i;
        x[i] = 15.0 * sin((double)i * 0.1);
    }
    sentil_trace_t *tr = sentil_trace_create(times, n);
    sentil_trace_add_signal(tr, "x", x, n);
    free(times);
    free(x);
    return tr;
}

static const char *FORMULA = "always[0, 100](eventually[0, 10](x > 5))";

static void scalability(void) {
    uint64_t sizes[] = {1000, 10000, 100000, 1000000, 10000000};
    for (int s = 0; s < 5; s++) {
        size_t n = (size_t)sizes[s];
        uint64_t runs = n <= 100000 ? 30 : 5;
        sentil_trace_t *tr = oracle_trace(n);
        sentil_monitor_t *m = sentil_monitor_parse(FORMULA, NULL);
        double *samples = malloc(runs * sizeof(double));

        size_t len = 0;
        double *sig = sentil_monitor_robustness_signal(m, tr, &len);
        double full_rob = sig[0];
        sentil_free_doubles(sig, len);
        for (uint64_t r = 0; r < runs; r++) {
            double t0 = now_ms();
            size_t l = 0;
            double *out = sentil_monitor_robustness_signal(m, tr, &l);
            sentil_free_doubles(out, l);
            samples[r] = now_ms() - t0;
        }
        emit("scalability/length", FORMULA, "full_signal", n, full_rob, summarize(samples, runs),
             runs);

        double mon_rob = 0.0;
        sentil_monitor_robustness(m, tr, &mon_rob);
        for (uint64_t r = 0; r < runs; r++) {
            double t0 = now_ms();
            double out = 0.0;
            sentil_monitor_robustness(m, tr, &out);
            samples[r] = now_ms() - t0;
        }
        emit("scalability/length", FORMULA, "monitoring", n, mon_rob, summarize(samples, runs),
             runs);

        free(samples);
        sentil_monitor_destroy(m);
        sentil_trace_destroy(tr);
    }
}

static void streaming(void) {
    sentil_stream_monitor_t *m = sentil_stream_monitor_create(FORMULA);
    size_t idx = 0;
    bool found = false;
    sentil_stream_monitor_symbol_index(m, "x", &idx, &found);
    size_t n = 1000000;
    double *latencies = malloc(n * sizeof(double));
    double packed[1] = {0.0};
    double last = 0.0;
    for (size_t i = 0; i < n; i++) {
        packed[idx] = 15.0 * sin((double)i * 0.1);
        sentil_robustness_t out;
        double t0 = now_ms();
        sentil_stream_monitor_update_packed(m, (double)i, packed, 1, &out);
        latencies[i] = now_ms() - t0;
        last = out.lower;
    }
    emit("streaming", FORMULA, "monitoring", n, last, summarize(latencies, n), n);
    free(latencies);
    sentil_stream_monitor_destroy(m);
}

int main(int argc, char **argv) {
    const char *suite = argc > 1 ? argv[1] : "";
    if (strcmp(suite, "scalability") == 0) {
        scalability();
    } else if (strcmp(suite, "streaming") == 0) {
        streaming();
    } else {
        fprintf(stderr, "unknown suite `%s`; use `scalability` or `streaming`\n", suite);
        return 1;
    }
    return 0;
}