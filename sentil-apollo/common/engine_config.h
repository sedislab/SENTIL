#pragma once

#include <cstddef>

#include "sentil/sentil.hpp"

#include "modules/sentil/proto/sentil_config.pb.h"
#include "modules/sentil/proto/sentil_control_config.pb.h"

namespace apollo {
namespace sentil {

::sentil::NoiseModel noise_from_proto(const NoiseModel& proto);

::sentil::LiftingRegistry lifting_from_proto(const SentilConfig& config);

::sentil::SmcConfig smc_from_proto(const SentilConfig& config);

::sentil::Bounds bounds_from_proto(const Bounds& proto);

::sentil::SystemModel model_from_proto(const LinearModel& proto, std::size_t input_width);

::sentil::Formula formula_from_spec(const Spec& spec);

::sentil::Bounds tile_bounds(const Bounds& proto, std::size_t input_width, std::size_t horizon);

::sentil::StochasticSystem chance_system_from_model(const LinearModel& proto, double process_std);

}  // namespace sentil
}  // namespace apollo