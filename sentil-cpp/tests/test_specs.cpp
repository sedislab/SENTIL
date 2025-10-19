#include <sentil/sentil.hpp>

#include <string>
#include <vector>

#include "sentil_test.hpp"

using sentil::Formula;
using sentil::SpecBuilder;

int main() {
    std::vector<std::string> names = SpecBuilder::available();
    CHECK(!names.empty());

    const std::string& first = names.front();
    CHECK(!SpecBuilder(first).build_deterministic().empty());
    CHECK(SpecBuilder(first).build_formula().depth() >= 1);

    std::string parameters = SpecBuilder(first).parameters_json();
    CHECK(!parameters.empty() && parameters.front() == '{');

    sentil::Monitor monitor = SpecBuilder(first).build_monitor();
    CHECK(!monitor.formula().variables().empty() || monitor.formula().depth() >= 1);

    return sentil_report("test_specs");
}