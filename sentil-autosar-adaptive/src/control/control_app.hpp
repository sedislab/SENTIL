#ifndef SENTIL_AP_CONTROL_APP_HPP
#define SENTIL_AP_CONTROL_APP_HPP

#include <memory>
#include <string>
#include <vector>

#include "sentil/sentil.hpp"

#include "sentil_ap/payloads.h"

namespace sentil_ap {

/// Owns the SENTIL controller and safety filter and answers a ComputeControl request. Two
/// modes: SHIELD projects a nominal command into the bounds and barriers (least
/// restrictive), and SYNTHESIZE plans a command from the spec under a deadline. The
/// transport is separate: the app main offers this over ara::com.
class ControlApp {
 public:
  struct Outcome {
    std::vector<double> command;
    ControllerStatus status;
  };

  static ControlApp shield(const std::vector<double>& lower, const std::vector<double>& upper);

  /// A receding-horizon controller over a linear model and a spec, bounded per step.
  static ControlApp synthesize(const std::vector<std::vector<double>>& a,
                               const std::vector<std::vector<double>>& b,
                               const std::vector<double>& x0,
                               const std::vector<std::string>& variables, double dt,
                               std::size_t horizon, const std::string& spec,
                               const std::vector<double>& lower,
                               const std::vector<double>& upper, std::uint64_t budget_ns);

  /// Compute a command from the current state and the nominal command. SHIELD reads
  /// nominal; SYNTHESIZE reads state. Throws a SentilError on an engine failure, which the
  /// app main maps to an infeasible outcome and a DEM event.
  Outcome compute(const std::vector<double>& state, const std::vector<double>& nominal);

 private:
  ControlApp() = default;

  std::unique_ptr<::sentil::SafetyFilter> shield_;
  std::unique_ptr<::sentil::Controller> controller_;
};

}  // namespace sentil_ap

#endif  // SENTIL_AP_CONTROL_APP_HPP