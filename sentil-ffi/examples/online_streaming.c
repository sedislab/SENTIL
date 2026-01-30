/* Online streaming: fold one timestamped sample at a time. */
#include "sentil.h"
#include <math.h>
#include <stdio.h>

int main(void) {
    sentil_monitor_t *monitor = sentil_monitor_parse("always[0, 10] (x > -0.9)", NULL);
    if (monitor == NULL) {
        fprintf(stderr, "parse error: %s\n", sentil_get_last_error());
        return 1;
    }

    const char *names[] = {"x"};
    for (int t = 0; t < 60; ++t) {
        double x = sin(t * 0.3);
        sentil_robustness_t out;
        if (sentil_monitor_update(monitor, (double)t, names, &x, 1, &out) != SENTIL_OK) {
            fprintf(stderr, "update error: %s\n", sentil_get_last_error());
            sentil_monitor_destroy(monitor);
            return 1;
        }
        if (out.resolved && !out.satisfied) {
            printf("violated at t=%d, robustness=%.3f\n", t, out.value);
            sentil_monitor_destroy(monitor);
            return 0;
        }
    }

    printf("held over the whole stream\n");
    sentil_monitor_destroy(monitor);
    return 0;
}