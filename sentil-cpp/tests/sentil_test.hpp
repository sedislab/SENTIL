#ifndef SENTIL_TEST_HPP
#define SENTIL_TEST_HPP

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>

inline int& sentil_checks_failed() {
    static int failed = 0;
    return failed;
}

inline void sentil_check(bool condition, const char* expr, const char* file, int line) {
    if (!condition) {
        std::fprintf(stderr, "FAIL %s:%d: %s\n", file, line, expr);
        ++sentil_checks_failed();
    }
}

inline bool sentil_same_bits(double a, double b) {
    std::uint64_t ua;
    std::uint64_t ub;
    std::memcpy(&ua, &a, sizeof(ua));
    std::memcpy(&ub, &b, sizeof(ub));
    return ua == ub;
}

#define CHECK(cond) sentil_check((cond), #cond, __FILE__, __LINE__)
#define CHECK_BITS(a, b) \
    sentil_check(sentil_same_bits((a), (b)), #a " bit-equals " #b, __FILE__, __LINE__)
#define CHECK_CLOSE(a, b, eps)                                                          \
    sentil_check(std::fabs((double)(a) - (double)(b)) <= (eps), #a " ~= " #b, __FILE__, \
                 __LINE__)

inline int sentil_report(const char* name) {
    if (sentil_checks_failed() > 0) {
        std::fprintf(stderr, "%s: %d checks failed\n", name, sentil_checks_failed());
        return 1;
    }
    std::printf("%s ok\n", name);
    return 0;
}

#endif  // SENTIL_TEST_HPP