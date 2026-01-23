#ifndef SENTIL_AP_MONITOR_APP_HPP
#define SENTIL_AP_MONITOR_APP_HPP

#include <map>
#include <memory>
#include <string>
#include <vector>

#include "sentil/sentil.hpp"

#include "sentil_ap/payloads.h"

namespace sentil_ap {

class MonitorApp {
 public:
  MonitorApp();

  void add(const std::string& id, const std::string& formula);

  void add_probabilistic(const std::string& id, const std::string& formula,
                         const std::string& variable, ::sentil::NoiseModel noise,
                         ::sentil::NoiseInteraction interaction, double confidence,
                         std::uint64_t samples);

  /// Replace the monitored formulas with a single deterministic one, validating it first.
  /// Returns false and leaves the monitor unchanged when the formula does not parse. This
  /// backs the SetSpecification control-plane method.
  bool set_specification(const std::string& formula);

  std::map<std::string, Verdict> on_frame(const SignalFrame& frame);

  const std::vector<std::string>& ids() const { return ids_; }

 private:
  Verdict to_verdict(double time, const std::string& id, const ::sentil::Robustness& robustness);

  std::unique_ptr<::sentil::MultiMonitor> monitor_;
  ::sentil::LiftingRegistry lifting_;
  std::vector<std::string> ids_;
  std::map<std::string, double> confidence_;
  std::map<std::string, std::uint64_t> samples_;
};

}  // namespace sentil_ap

#endif  // SENTIL_AP_MONITOR_APP_HPP