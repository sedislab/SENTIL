#include <sentil/sentil.hpp>

#include <iostream>

int main() {
    sentil::Trace trace({0, 1, 2, 3, 4}, "speed", {12.0, 9.0, 7.0, 4.0, 6.0});
    sentil::Formula phi = sentil::Formula::parse("always (speed > 5)");

    std::cout << "robustness: " << phi.robustness(trace) << "\n";
    std::cout << "per sample:";
    for (double r : phi.robustness_signal(trace)) {
        std::cout << " " << r;
    }
    std::cout << "\n";
    std::cout << "violations:";
    for (const sentil::Interval& v : phi.violations(trace)) {
        std::cout << " [" << v.start << ", " << v.end << "]";
    }
    std::cout << "\n";
    std::cout << "dense robustness: " << phi.robustness_dense(trace) << "\n";
    return 0;
}