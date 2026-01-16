#include <sentil/sentil.hpp>

#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <limits>
#include <map>
#include <sstream>
#include <string>
#include <vector>

#include "json.hpp"
#include "sentil_test.hpp"

#include "modules/sentil/common/engine_config.h"
#include "modules/sentil/proto/sentil_config.pb.h"
#include "modules/sentil/proto/sentil_control_config.pb.h"

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

static bool reproduce_oracle() {
  std::ifstream in(SENTIL_ORACLE_PATH);
  if (!in.good()) {
    std::fprintf(stderr, "cannot open oracle at %s\n", SENTIL_ORACLE_PATH);
    return false;
  }
  std::stringstream buffer;
  buffer << in.rdbuf();
  testjson::Value root = testjson::parse(buffer.str());
  const testjson::Value& cases = root["deterministic"];
  int reproduced = 0;
  for (std::size_t ci = 0; ci < cases.size(); ++ci) {
    const testjson::Value& test = cases[ci];
    const std::size_t length = static_cast<std::size_t>(test["length"].number);
    sentil::Trace trace = sentil::Trace::indexed(length);
    const testjson::Value& signals = test["signals"];
    for (std::size_t si = 0; si < signals.size(); ++si) {
      trace.add_signal(signals[si]["name"].text, parse_values(signals[si]["values"]));
    }
    const std::vector<double> expected = parse_values(test["expected"]);
    const std::vector<double> got = sentil::Formula::parse(test["formula"].text).robustness_signal(trace);
    CHECK(got.size() == expected.size());
    for (std::size_t i = 0; i < got.size() && i < expected.size(); ++i) {
      CHECK_BITS(got[i], expected[i]);
    }
    ++reproduced;
  }
  CHECK(reproduced >= 44);
  return true;
}

static void check_engine_bridge() {
  apollo::sentil::SentilConfig config;
  config.set_sample_budget(1000);
  config.set_confidence(0.95);
  auto* entry = config.add_lifting_registry();
  entry->set_variable("gap");
  entry->mutable_noise()->mutable_gaussian()->set_std_dev(0.25);

  sentil::LiftingRegistry registry = apollo::sentil::lifting_from_proto(config);
  CHECK(registry.variables().size() == 1);
  CHECK(registry.variables()[0] == "gap");

  sentil::SmcConfig smc = apollo::sentil::smc_from_proto(config);
  CHECK(smc.samples == 1000);
  CHECK_CLOSE(smc.confidence, 0.95, 1e-12);

  sentil::MultiMonitor monitor;
  monitor.add("gt", "x > 0");
  const double xs[] = {5.0, -3.0, 2.0};
  for (int i = 0; i < 3; ++i) {
    std::map<std::string, double> sample = {{"x", xs[i]}};
    std::map<std::string, sentil::Robustness> verdicts = monitor.update(static_cast<double>(i), sample);
    CHECK_BITS(verdicts.at("gt").value, xs[i]);
  }

  apollo::sentil::LinearModel model_proto;
  model_proto.add_a(1.0);
  model_proto.add_b(1.0);
  model_proto.add_x0(1.0);
  model_proto.add_variables("x");
  model_proto.set_dt(1.0);
  model_proto.set_horizon(3);
  sentil::SystemModel model = apollo::sentil::model_from_proto(model_proto, 1);
  sentil::Formula spec = sentil::Formula::parse("always (x > 0)");
  sentil::Bounds bounds({-1.0, -1.0, -1.0}, {1.0, 1.0, 1.0});
  sentil::SynthesisResult result = sentil::synthesis::synthesize(model, spec, &bounds);
  CHECK(result.holds);
}

int main() {
  const bool oracle = reproduce_oracle();
  check_engine_bridge();
  return oracle ? sentil_report("oracle_parity") : 1;
}