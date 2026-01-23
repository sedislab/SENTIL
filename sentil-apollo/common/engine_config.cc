#include "modules/sentil/common/engine_config.h"

#include <cstdint>
#include <random>
#include <stdexcept>
#include <string>
#include <vector>

namespace apollo {
namespace sentil {
namespace {

::sentil::NoiseInteraction interaction_of(NoiseInteraction proto) {
  return proto == MULTIPLICATIVE ? ::sentil::NoiseInteraction::Multiplicative
                                 : ::sentil::NoiseInteraction::Additive;
}

std::vector<std::vector<double>> reshape(const google::protobuf::RepeatedField<double>& flat,
                                         std::size_t rows, std::size_t cols, const char* what) {
  if (static_cast<std::size_t>(flat.size()) != rows * cols) {
    throw std::invalid_argument(std::string(what) + " must have " + std::to_string(rows * cols) +
                                " entries, got " + std::to_string(flat.size()));
  }
  std::vector<std::vector<double>> out(rows, std::vector<double>(cols));
  for (std::size_t i = 0; i < rows; ++i) {
    for (std::size_t j = 0; j < cols; ++j) {
      out[i][j] = flat.Get(i * cols + j);
    }
  }
  return out;
}

}  // namespace

::sentil::NoiseModel noise_from_proto(const NoiseModel& proto) {
  switch (proto.model_case()) {
    case NoiseModel::kDirac:
      return ::sentil::NoiseModel::dirac(proto.dirac());
    case NoiseModel::kGaussian:
      return ::sentil::NoiseModel::gaussian(proto.gaussian().mean(), proto.gaussian().std_dev());
    case NoiseModel::kUniform:
      return ::sentil::NoiseModel::uniform(proto.uniform().low(), proto.uniform().high());
    case NoiseModel::kLogNormal:
      return ::sentil::NoiseModel::log_normal(proto.log_normal().mu(), proto.log_normal().sigma());
    case NoiseModel::kExponential:
      return ::sentil::NoiseModel::exponential(proto.exponential().rate());
    case NoiseModel::kGamma:
      return ::sentil::NoiseModel::gamma(proto.gamma().shape(), proto.gamma().scale());
    case NoiseModel::kBeta:
      return ::sentil::NoiseModel::beta(proto.beta().alpha(), proto.beta().beta());
    case NoiseModel::kTruncatedNormal:
      return ::sentil::NoiseModel::truncated_normal(
          proto.truncated_normal().mean(), proto.truncated_normal().std_dev(),
          proto.truncated_normal().lower(), proto.truncated_normal().upper());
    case NoiseModel::MODEL_NOT_SET:
      break;
  }
  throw std::invalid_argument("a lifting entry has no noise model set");
}

::sentil::LiftingRegistry lifting_from_proto(const SentilConfig& config) {
  ::sentil::LiftingRegistry registry;
  for (const LiftingEntry& entry : config.lifting_registry()) {
    registry.register_noise(entry.variable(), noise_from_proto(entry.noise()),
                            interaction_of(entry.interaction()));
  }
  return registry;
}

::sentil::SmcConfig smc_from_proto(const SentilConfig& config) {
  ::sentil::SmcConfig smc;
  smc.samples = config.sample_budget();
  smc.confidence = config.confidence();
  return smc;
}

::sentil::Bounds bounds_from_proto(const Bounds& proto) {
  std::vector<double> lower(proto.lower().begin(), proto.lower().end());
  std::vector<double> upper(proto.upper().begin(), proto.upper().end());
  return ::sentil::Bounds(lower, upper);
}

::sentil::SystemModel model_from_proto(const LinearModel& proto, std::size_t input_width) {
  const std::size_t n = proto.variables_size();
  if (n == 0) {
    throw std::invalid_argument("model has no state variables");
  }
  auto a = reshape(proto.a(), n, n, "model.a");
  auto b = reshape(proto.b(), n, input_width, "model.b");
  std::vector<double> x0(proto.x0().begin(), proto.x0().end());
  if (x0.size() != n) {
    throw std::invalid_argument("model.x0 must have " + std::to_string(n) + " entries");
  }
  std::vector<std::string> variables(proto.variables().begin(), proto.variables().end());
  return ::sentil::SystemModel::linear(a, b, x0, variables, proto.dt(), proto.horizon());
}

::sentil::Formula formula_from_spec(const Spec& spec) {
  if (!spec.expression().empty()) {
    return ::sentil::Formula::parse(spec.expression());
  }
  if (!spec.name().empty()) {
    ::sentil::SpecBuilder builder(spec.name());
    if (!spec.variant().empty()) {
      builder = std::move(builder).with_variant(spec.variant());
    }
    return builder.build_formula();
  }
  throw std::invalid_argument("control spec has neither an expression nor a name");
}

::sentil::Bounds tile_bounds(const Bounds& proto, std::size_t input_width, std::size_t horizon) {
  if (proto.lower_size() != static_cast<int>(input_width) ||
      proto.upper_size() != static_cast<int>(input_width)) {
    throw std::invalid_argument("control bounds need input_width lower and upper entries per step");
  }
  std::vector<double> lower;
  std::vector<double> upper;
  lower.reserve(horizon * input_width);
  upper.reserve(horizon * input_width);
  for (std::size_t step = 0; step < horizon; ++step) {
    for (std::size_t i = 0; i < input_width; ++i) {
      lower.push_back(proto.lower(i));
      upper.push_back(proto.upper(i));
    }
  }
  return ::sentil::Bounds(lower, upper);
}

::sentil::StochasticSystem chance_system_from_model(const LinearModel& proto, double process_std) {
  const std::size_t n = proto.variables_size();
  if (n == 0) {
    throw std::invalid_argument("model has no state variables");
  }
  const std::vector<std::vector<double>> a = reshape(proto.a(), n, n, "model.a");
  const std::vector<double> x0(proto.x0().begin(), proto.x0().end());
  if (x0.size() != n) {
    throw std::invalid_argument("model.x0 must have " + std::to_string(n) + " entries");
  }
  std::vector<std::string> variables(proto.variables().begin(), proto.variables().end());
  return ::sentil::StochasticSystem::custom(
      variables, proto.dt(), proto.horizon(), [x0](std::uint64_t) { return x0; },
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
}

}  // namespace sentil
}  // namespace apollo