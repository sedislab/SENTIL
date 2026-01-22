#include <sentil/sentil.hpp>

#include <cstdlib>
#include <fstream>
#include <limits>
#include <map>
#include <sstream>
#include <string>
#include <vector>

#include "json.hpp"
#include "sentil_test.hpp"

#include "monitor_app.hpp"
#include "sentil_ap/payloads.h"

#ifndef SENTIL_ORACLE_PATH
#define SENTIL_ORACLE_PATH "../../benchmarks/deterministic/oracle.json"
#endif

static double parse_token(const std::string& token) {
  if (token == "inf") return std::numeric_limits<double>::infinity();
  if (token == "-inf") return -std::numeric_limits<double>::infinity();
  if (token == "nan") return std::numeric_limits<double>::quiet_NaN();
  return std::strtod(token.c_str(), nullptr);
}

static std::vector<double> parse_values(const testjson::Value& array) {
  std::vector<double> out;
  out.reserve(array.size());
  for (std::size_t i = 0; i < array.size(); ++i) {
    out.push_back(parse_token(array[i].text));
  }
  return out;
}

int main() {
  std::ifstream in(SENTIL_ORACLE_PATH);
  if (!in.good()) {
    std::fprintf(stderr, "cannot open oracle at %s\n", SENTIL_ORACLE_PATH);
    return 1;
  }
  std::stringstream buffer;
  buffer << in.rdbuf();
  testjson::Value root = testjson::parse(buffer.str());
  const testjson::Value& cases = root["deterministic"];

  sentil_ap::Verdict probe;
  probe.robustness_min = std::numeric_limits<double>::infinity();
  probe.robustness_max = -2.5;
  probe.satisfied = true;
  probe.is_concrete = false;
  probe.probability = 0.875;
  const sentil_ap::Verdict echoed = sentil_ap::parse_verdict(sentil_ap::serialize(probe));
  CHECK_BITS(echoed.robustness_min, probe.robustness_min);
  CHECK_BITS(echoed.robustness_max, probe.robustness_max);
  CHECK(echoed.satisfied == probe.satisfied);
  CHECK(echoed.is_concrete == probe.is_concrete);
  CHECK_BITS(echoed.probability, probe.probability);

  int reproduced = 0;
  for (std::size_t ci = 0; ci < cases.size(); ++ci) {
    const testjson::Value& test = cases[ci];
    const std::string id = test["id"].text;
    const std::string formula = test["formula"].text;
    const std::size_t length = static_cast<std::size_t>(test["length"].number);
    const testjson::Value& signals = test["signals"];

    sentil::Trace trace = sentil::Trace::indexed(length);
    std::vector<std::string> names;
    std::vector<std::vector<double>> series;
    for (std::size_t si = 0; si < signals.size(); ++si) {
      names.push_back(signals[si]["name"].text);
      series.push_back(parse_values(signals[si]["values"]));
      trace.add_signal(names.back(), series.back());
    }
    const std::vector<double> expected = parse_values(test["expected"]);
    const std::vector<double> got = sentil::Formula::parse(formula).robustness_signal(trace);
    CHECK(got.size() == expected.size());
    for (std::size_t i = 0; i < got.size() && i < expected.size(); ++i) {
      CHECK_BITS(got[i], expected[i]);
    }

    sentil::MultiMonitor reference;
    reference.add(id, formula);
    sentil_ap::MonitorApp app;
    app.add(id, formula);
    for (std::size_t t = 0; t < length; ++t) {
      sentil_ap::SignalFrame frame;
      frame.t = static_cast<double>(t);
      std::map<std::string, double> values;
      for (std::size_t si = 0; si < names.size(); ++si) {
        frame.names.push_back(names[si]);
        frame.values.push_back(series[si][t]);
        values[names[si]] = series[si][t];
      }
      const sentil::Robustness expected_robustness = reference.update(frame.t, values).at(id);
      const auto verdicts = app.on_frame(frame);
      const sentil_ap::Verdict served = sentil_ap::parse_verdict(sentil_ap::serialize(verdicts.at(id)));
      CHECK_BITS(served.robustness_min,
                 expected_robustness.resolved ? expected_robustness.value : expected_robustness.lower);
    }
    ++reproduced;
  }

  std::printf("parity: reproduced %d deterministic cases through the served verdict\n", reproduced);
  CHECK(reproduced >= 44);
  return sentil_report("parity");
}