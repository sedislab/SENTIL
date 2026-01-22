#include "control_app.hpp"

#include <cmath>

namespace sentil_ap {

ControlApp ControlApp::shield(const std::vector<double>& lower, const std::vector<double>& upper) {
  ControlApp app;
  app.shield_ = std::make_unique<::sentil::SafetyFilter>(::sentil::Bounds(lower, upper));
  return app;
}

ControlApp ControlApp::synthesize(const std::vector<std::vector<double>>& a,
                                  const std::vector<std::vector<double>>& b,
                                  const std::vector<double>& x0,
                                  const std::vector<std::string>& variables, double dt,
                                  std::size_t horizon, const std::string& spec,
                                  const std::vector<double>& lower,
                                  const std::vector<double>& upper, std::uint64_t budget_ns) {
  ControlApp app;
  ::sentil::SystemModel model = ::sentil::SystemModel::linear(a, b, x0, variables, dt, horizon);
  ::sentil::Formula formula = ::sentil::Formula::parse(spec);
  const std::size_t input_width = variables.empty() ? 1 : b.front().size();
  // Open-loop and receding-horizon planning bound the whole input sequence, so tile the
  // per-step limits across the horizon.
  std::vector<double> seq_lower;
  std::vector<double> seq_upper;
  for (std::size_t step = 0; step < horizon; ++step) {
    seq_lower.insert(seq_lower.end(), lower.begin(), lower.end());
    seq_upper.insert(seq_upper.end(), upper.begin(), upper.end());
  }
  ::sentil::Bounds bounds(seq_lower, seq_upper);
  app.controller_ = std::make_unique<::sentil::Controller>(std::move(model), std::move(formula),
                                                          input_width, budget_ns, &bounds);
  return app;
}

ControlApp::Outcome ControlApp::compute(const std::vector<double>& state,
                                        const std::vector<double>& nominal) {
  Outcome outcome;
  outcome.status.deadline_met = true;
  outcome.status.feasible = true;
  if (shield_) {
    outcome.command = shield_->filter(nominal);
    for (std::size_t i = 0; i < outcome.command.size() && i < nominal.size(); ++i) {
      if (outcome.command[i] != nominal[i]) {
        outcome.status.cbf_active = true;
        break;
      }
    }
  } else {
    outcome.command = controller_->control(state);
  }
  return outcome;
}

}  // namespace sentil_ap