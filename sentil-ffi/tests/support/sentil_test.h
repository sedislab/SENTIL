#ifndef SENTIL_TEST_H
#define SENTIL_TEST_H

#include "sentil.h"
#include <math.h>
#include <stdio.h>
#include <string.h>

static int sentil_checks_failed = 0;

#define CHECK(cond)                                                         \
    do {                                                                    \
        if (!(cond)) {                                                      \
            fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond); \
            sentil_checks_failed++;                                         \
        }                                                                   \
    } while (0)

#define CHECK_CLOSE(a, b, eps) CHECK(fabs((double)(a) - (double)(b)) < (eps))

static inline int sentil_report(const char *name) {
    if (sentil_checks_failed != 0) {
        fprintf(stderr, "%s: %d checks failed\n", name, sentil_checks_failed);
        return 1;
    }
    printf("%s ok\n", name);
    return 0;
}

#endif /* SENTIL_TEST_H */