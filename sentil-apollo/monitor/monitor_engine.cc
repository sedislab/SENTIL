#include "modules/sentil/monitor/monitor_engine.h"

#include <chrono>
#include <cmath>
#include <optional>
#include <vector>

#include "cyber/time/clock.h"

#include "modules/sentil/common/engine_config.h"

namespace apollo {
namespace sentil {

void MonitorEngine::Build(const SentilConfig& config) {
  config_ = config;
  monitor_ = std::make_unique<::sentil::MultiMonitor>();
  ::sentil::LiftingRegistry lifting = lifting_from_proto(config_);
  ::sentil::SmcConfig smc = smc_from_proto(config_);
  for (const Formula& formula : config_.formulas()) {
    const std::string id = std::to_string(formula.id());
    const std::string& expression = formula.expression();
    const bool probabilistic = !expression.empty() && expression[0] == 'P';
    if (probabilistic) {
      ::sentil::Formula parsed = ::sentil::Formula::parse(expression);
      monitor_->add_probabilistic(id, parsed, lifting, smc);
      confidence_[id] = config_.confidence();
    } else {
      monitor_->add(id, expression);
    }
  }
  for (const ChannelMapping& channel : config_.input_channels()) {
    std::vector<FieldMapping> fields(channel.fields().begin(), channel.fields().end());
    extractor_.add_channel(channel.message_type(), fields);
  }
}

bool MonitorEngine::Evaluate(const apollo::perception::PerceptionObstacles& perception,
                             const apollo::localization::LocalizationEstimate& localization,
                             const apollo::canbus::Chassis& chassis, SentilStatus* status) {
  const auto start = std::chrono::steady_clock::now();

  std::vector<std::string> names;
  std::vector<double> values;
  extractor_.extract_into(perception.GetTypeName(), perception, &names, &values);
  extractor_.extract_into(localization.GetTypeName(), localization, &names, &values);
  extractor_.extract_into(chassis.GetTypeName(), chassis, &names, &values);

  const double stamp = perception.has_header() && perception.header().has_timestamp_sec()
                           ? perception.header().timestamp_sec()
                           : apollo::cyber::Clock::NowInSeconds();
  if (stamp <= last_stamp_) {
    return false;
  }
  last_stamp_ = stamp;

  std::map<std::string, double> sample;
  for (std::size_t i = 0; i < names.size(); ++i) {
    sample[names[i]] = values[i];
  }

  std::map<std::string, ::sentil::Robustness> verdicts = monitor_->update(stamp, sample);
  if (verdicts.empty()) {
    return false;
  }

  status->mutable_header()->set_timestamp_sec(stamp);
  bool all_satisfied = true;
  for (const Formula& formula : config_.formulas()) {
    const auto it = verdicts.find(std::to_string(formula.id()));
    if (it == verdicts.end()) {
      continue;
    }
    FormulaResult* result = status->add_results();
    result->set_id(formula.id());
    result->set_expression(formula.expression());
    FillResult(formula, it->second, result);
    all_satisfied = all_satisfied && result->satisfied();
  }
  status->set_all_satisfied(all_satisfied);
  const auto elapsed = std::chrono::steady_clock::now() - start;
  status->set_computation_time_ms(std::chrono::duration<double, std::milli>(elapsed).count());
  return true;
}

void MonitorEngine::FillResult(const Formula& formula, const ::sentil::Robustness& robustness,
                               FormulaResult* result) {
  Robustness* out = result->mutable_robustness();
  out->set_is_concrete(robustness.resolved);
  out->set_min(robustness.resolved ? robustness.value : robustness.lower);
  out->set_max(robustness.resolved ? robustness.value : robustness.upper);
  result->set_satisfied(robustness.satisfied);
  result->set_severity(!robustness.resolved ? WARN : (robustness.satisfied ? OK : ERROR));

  const std::string id = std::to_string(formula.id());
  const std::optional<double> probability = monitor_->probability(id);
  if (!probability) {
    return;
  }
  const std::uint64_t samples = config_.sample_budget();
  const double confidence = confidence_.at(id);
  ProbabilisticResult* prob = result->mutable_prob_result();
  prob->set_samples(samples);
  prob->set_probability(*probability);
  ConfidenceInterval* interval = prob->mutable_interval();
  interval->set_confidence_level(confidence);
  if (!robustness.resolved) {
    prob->set_satisfactions(0);
    interval->set_lower(0.0);
    interval->set_upper(1.0);
    return;
  }
  const std::uint64_t satisfactions =
      static_cast<std::uint64_t>(std::llround(*probability * static_cast<double>(samples)));
  prob->set_satisfactions(satisfactions);
  const ::sentil::ConfidenceInterval ci =
      ::sentil::stats::wilson_interval(satisfactions, samples, confidence);
  interval->set_lower(ci.lower);
  interval->set_upper(ci.upper);
}

}  // namespace sentil
}  // namespace apollo