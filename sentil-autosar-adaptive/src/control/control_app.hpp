#ifndef SENTIL_AP_CONTROL_APP_HPP
#define SENTIL_AP_CONTROL_APP_HPP

#include <memory>
#include <string>
#include <vector>

#include "sentil/sentil.hpp"

#include "sentil_ap/payloads.h"

namespace sentil_ap {

class ControlApp {
 public:
  struct Outcome {
    std::vector<double> command;
    ControllerStatus status;
  };

  static ControlApp shield(const std::vector<double>& lower, const std::vector<double>& upper);

  static ControlApp synthesize(const std::vector<std::vector<double>>& a,
                               const std::vector<std::vector<double>>& b,
                               const std::vector<double>& x0,
                               const std::vector<std::string>& variables, double dt,
                               std::size_t horizon, const std::string& spec,
                               const std::vector<double>& lower,
                               const std::vector<double>& upper, std::uint64_t budget_ns);

  Outcome compute(const std::vector<double>& state, const std::vector<double>& nominal);

  ::sentil::SynthesisResult plan() const;

  ::sentil::Witness falsify() const;

  ::sentil::ChanceReport validate_chance(double probability, double confidence,
                                         double process_std) const;

 private:
  ControlApp() = default;
  ::sentil::SystemModel build_model() const;
  ::sentil::Bounds sequence_bounds() const;

  std::unique_ptr<::sentil::SafetyFilter> shield_;
  std::unique_ptr<::sentil::Controller> controller_;

  bool has_problem_ = false;
  std::vector<std::vector<double>> a_;
  std::vector<std::vector<double>> b_;
  std::vector<double> x0_;
  std::vector<double> lower_;
  std::vector<double> upper_;
  std::vector<std::string> variables_;
  double dt_ = 0.1;
  std::size_t horizon_ = 0;
  std::size_t input_width_ = 1;
  std::string spec_;
};

}  // namespace sentil_ap

#endif  // SENTIL_AP_CONTROL_APP_HPP