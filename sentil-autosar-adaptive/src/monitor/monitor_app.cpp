#include "monitor_app.hpp"

#include <cmath>
#include <optional>
#include <utility>

namespace sentil_ap {

MonitorApp::MonitorApp() : monitor_(std::make_unique<::sentil::MultiMonitor>()) {}

void MonitorApp::add(const std::string& id, const std::string& formula) {
  std::lock_guard<std::mutex> guard(mutex_);
  add_unlocked(id, formula);
}

void MonitorApp::add_unlocked(const std::string& id, const std::string& formula) {
  monitor_->add(id, formula);
  ids_.push_back(id);
}

void MonitorApp::add_probabilistic(const std::string& id, const std::string& formula,
                                   const std::string& variable, ::sentil::NoiseModel noise,
                                   ::sentil::NoiseInteraction interaction, double confidence,
                                   std::uint64_t samples) {
  std::lock_guard<std::mutex> guard(mutex_);
  lifting_.register_noise(variable, std::move(noise), interaction);
  ::sentil::Formula parsed = ::sentil::Formula::parse(formula);
  ::sentil::SmcConfig config;
  config.samples = samples;
  config.confidence = confidence;
  monitor_->add_probabilistic(id, parsed, lifting_, config);
  ids_.push_back(id);
  confidence_[id] = confidence;
  samples_[id] = samples;
}

bool MonitorApp::set_specification(const std::string& formula, std::string& error) {
  try {
    ::sentil::Formula::parse(formula);
  } catch (const std::exception& parse_error) {
    error = parse_error.what();
    return false;
  }
  std::lock_guard<std::mutex> guard(mutex_);
  monitor_ = std::make_unique<::sentil::MultiMonitor>();
  ids_.clear();
  confidence_.clear();
  samples_.clear();
  add_unlocked("spec", formula);
  return true;
}

std::map<std::string, Verdict> MonitorApp::on_frame(const SignalFrame& frame) {
  std::lock_guard<std::mutex> guard(mutex_);
  std::map<std::string, double> values;
  for (std::size_t i = 0; i < frame.names.size() && i < frame.values.size(); ++i) {
    values[frame.names[i]] = frame.values[i];
  }
  std::map<std::string, Verdict> out;
  for (const auto& entry : monitor_->update(frame.t, values)) {
    out.emplace(entry.first, to_verdict(frame.t, entry.first, entry.second));
  }
  return out;
}

Verdict MonitorApp::to_verdict(double time, const std::string& id,
                               const ::sentil::Robustness& robustness) {
  Verdict verdict;
  verdict.timestamp = time;
  verdict.is_concrete = robustness.resolved;
  verdict.satisfied = robustness.satisfied;
  verdict.robustness_min = robustness.resolved ? robustness.value : robustness.lower;
  verdict.robustness_max = robustness.resolved ? robustness.value : robustness.upper;

  const std::optional<double> probability = monitor_->probability(id);
  if (!probability) {
    return verdict;
  }
  verdict.probability = *probability;
  if (!robustness.resolved) {
    verdict.ci_lower = 0.0;
    verdict.ci_upper = 1.0;
    return verdict;
  }
  const std::uint64_t samples = samples_.at(id);
  const auto satisfactions =
      static_cast<std::uint64_t>(std::llround(*probability * static_cast<double>(samples)));
  const ::sentil::ConfidenceInterval ci =
      ::sentil::stats::wilson_interval(satisfactions, samples, confidence_.at(id));
  verdict.ci_lower = ci.lower;
  verdict.ci_upper = ci.upper;
  return verdict;
}

}  // namespace sentil_ap