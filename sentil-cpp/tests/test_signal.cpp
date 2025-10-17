#include <sentil/sentil.hpp>

#include <cmath>
#include <vector>

#include "sentil_test.hpp"

using sentil::Interpolation;
using sentil::RingBuffer;
using sentil::Trace;

int main() {
    Trace t({0, 1, 2}, {{"x", {1, 2, 3}}, {"y", {4, 5, 6}}});
    CHECK(t.size() == 3 && !t.empty());
    CHECK(t.times() == std::vector<double>({0, 1, 2}));
    CHECK(t.variables() == std::vector<std::string>({"x", "y"}));
    CHECK(t.contains("x") && !t.contains("z"));
    CHECK(*t.signal("y") == std::vector<double>({4, 5, 6}));
    CHECK(!t.signal("z").has_value());
    CHECK(t["x"] == std::vector<double>({1, 2, 3}));

    Trace csv = Trace::from_csv("time,x,y\n0,1,4\n1,2,5\n2,3,6\n");
    CHECK(csv.size() == 3 && csv.contains("x"));

    Trace square({0, 1, 2, 3}, "x", {0, 1, 4, 9});
    Trace lin = square.resample({0.5, 1.5, 2.5}, Interpolation::Linear);
    CHECK_CLOSE((*lin.signal("x"))[1], 2.5, 1e-12);
    CHECK_CLOSE((*square.prepare(Interpolation::CubicSpline).resample({1.5}).signal("x"))[0], 2.2,
                1e-9);

    RingBuffer rb(3);
    CHECK(rb.capacity() == 3 && rb.empty());
    CHECK(!rb.push(0, 10).has_value());
    rb.push(1, 20);
    rb.push(2, 30);
    CHECK(rb.is_full());
    auto evicted = rb.push(3, 40);
    CHECK(evicted.has_value() && evicted->time == 0);
    CHECK(rb.front()->value == 20 && rb.back()->value == 40);
    CHECK(rb.get(1)->value == 30 && !rb.get(9).has_value());
    auto range = rb.time_range();
    CHECK(range.has_value() && range->first == 1 && range->second == 3);
    CHECK(rb.between(1, 2).size() == 2);

    RingBuffer stats(4);
    stats.push(0, 2);
    stats.push(1, 4);
    stats.push(2, 6);
    CHECK_CLOSE(*stats.mean(), 4.0, 1e-12);
    CHECK(*stats.min() == 2 && *stats.max() == 6);
    CHECK(stats.variance().has_value());

    return sentil_report("test_signal");
}