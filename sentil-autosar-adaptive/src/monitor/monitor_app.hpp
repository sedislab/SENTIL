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

  /// Add a P~p formula tracked online over an ensemble lifted with Gaussian noise on the
  /// named variable. Throws on a parse error.
  void add_probabilistic(const std::string& id, const std::string& formula,
                         const std::string& variable, double std_dev, double confidence,
                         std::uint64_t samples);

  /// Fold one frame and return the verdict for each formula, or an empty map until every
  /// monitored variable has been seen at least once.
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