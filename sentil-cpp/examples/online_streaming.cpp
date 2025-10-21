#include <sentil/sentil.hpp>

#include <cmath>
#include <cstdio>

int main() {
    sentil::OnlineMonitor monitor("always[0, 10] (x > -0.9)");
    for (int t = 0; t < 60; ++t) {
        double x = std::sin(t * 0.3);
        sentil::Robustness verdict = monitor.update(static_cast<double>(t), {{"x", x}});
        if (verdict.resolved && !verdict.satisfied) {
            std::printf("violated at t=%d, robustness=%.3f\n", t, verdict.value);
            return 0;
        }
    }
    std::printf("held over the whole stream\n");
    return 0;
}