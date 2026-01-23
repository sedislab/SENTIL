#include "control_app.hpp"

#include <random>
#include <stdexcept>

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
  app.has_problem_ = true;
  app.a_ = a;
  app.b_ = b;
  app.x0_ = x0;
  app.variables_ = variables;
  app.dt_ = dt;
  app.horizon_ = horizon;
  app.spec_ = spec;
  app.lower_ = lower;
  app.upper_ = upper;
  app.input_width_ = b.empty() ? 1 : b.front().size();
  ::sentil::Bounds bounds = app.sequence_bounds();
  app.controller_ = std::make_unique<::sentil::Controller>(
      app.build_model(), ::sentil::Formula::parse(spec), app.input_width_, budget_ns, &bounds);
  return app;
}

::sentil::SystemModel ControlApp::build_model() const {
  return ::sentil::SystemModel::linear(a_, b_, x0_, variables_, dt_, horizon_);
}

::sentil::Bounds ControlApp::sequence_bounds() const {
  std::vector<double> lower;
  std::vector<double> upper;
  for (std::size_t step = 0; step < horizon_; ++step) {
    lower.insert(lower.end(), lower_.begin(), lower_.end());
    upper.insert(upper.end(), upper_.begin(), upper_.end());
  }
  return ::sentil::Bounds(lower, upper);
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

::sentil::SynthesisResult ControlApp::plan() const {
  if (!has_problem_) {
    throw std::runtime_error("plan needs a synthesis problem, not a shield");
  }
  ::sentil::SystemModel model = build_model();
  ::sentil::Formula spec = ::sentil::Formula::parse(spec_);
  ::sentil::Bounds bounds = sequence_bounds();
  return ::sentil::synthesis::synthesize(model, spec, &bounds);
}

::sentil::Witness ControlApp::falsify() const {
  if (!has_problem_) {
    throw std::runtime_error("falsify needs a synthesis problem, not a shield");
  }
  ::sentil::SystemModel model = build_model();
  ::sentil::Bounds bounds = sequence_bounds();
  return ::sentil::Formula::parse(spec_).falsify(model, bounds);
}

::sentil::ChanceReport ControlApp::validate_chance(double probability, double confidence,
                                                   double process_std) const {
  if (!has_problem_) {
    throw std::runtime_error("chance check needs a synthesis problem, not a shield");
  }
  const std::vector<std::vector<double>> a = a_;
  const std::vector<double> x0 = x0_;
  const std::size_t n = variables_.size();
  ::sentil::StochasticSystem system = ::sentil::StochasticSystem::custom(
      variables_, dt_, horizon_, [x0](std::uint64_t) { return x0; },
      [a, n, process_std](const std::vector<double>& prev, double, std::uint64_t seed) {
        std::mt19937_64 rng(seed);
        std::normal_distribution<double> noise(0.0, process_std);
        std::vector<double> next(n, 0.0);
        for (std::size_t i = 0; i < n; ++i) {
          for (std::size_t j = 0; j < n && j < prev.size(); ++j) {
            next[i] += a[i][j] * prev[j];
          }
          next[i] += noise(rng);
        }
        return next;
      });
  ::sentil::ChanceConstraint constraint(::sentil::Formula::parse(spec_), probability, confidence);
  return constraint.validate(system);
}

}  // namespace sentil_ap