#include "modules/sentil/control/sentil_control_component.h"

#include <cstdint>
#include <map>

#include "modules/sentil/common/engine_config.h"

namespace apollo {
namespace sentil {
namespace {

constexpr char kAdvisoryChannel[] = "/apollo/sentil/control_advice";

const google::protobuf::FieldDescriptor* resolve_actuation_field(
    const google::protobuf::Descriptor* type, const std::string& name) {
  const google::protobuf::FieldDescriptor* field = type->FindFieldByName(name);
  if (field == nullptr) {
    throw std::invalid_argument("ControlCommand has no field '" + name + "'");
  }
  if (field->cpp_type() != google::protobuf::FieldDescriptor::CPPTYPE_DOUBLE &&
      field->cpp_type() != google::protobuf::FieldDescriptor::CPPTYPE_FLOAT) {
    throw std::invalid_argument("ControlCommand field '" + name + "' is not a floating-point actuator");
  }
  return field;
}

double get_actuation(const google::protobuf::Message& message,
                     const google::protobuf::FieldDescriptor* field) {
  const google::protobuf::Reflection* reflection = message.GetReflection();
  return field->cpp_type() == google::protobuf::FieldDescriptor::CPPTYPE_FLOAT
             ? reflection->GetFloat(message, field)
             : reflection->GetDouble(message, field);
}

void set_actuation(google::protobuf::Message* message,
                   const google::protobuf::FieldDescriptor* field, double value) {
  const google::protobuf::Reflection* reflection = message->GetReflection();
  if (field->cpp_type() == google::protobuf::FieldDescriptor::CPPTYPE_FLOAT) {
    reflection->SetFloat(message, field, static_cast<float>(value));
  } else {
    reflection->SetDouble(message, field, value);
  }
}

// Open-loop synthesis bounds the whole horizon-long input, so repeat the per-step limits
// across the horizon. Throws when the bounds are not one entry per input per step.
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

}  // namespace

bool SentilControlComponent::Init() {
  if (!GetProtoConfig(&config_)) {
    AERROR << "sentil control: could not load its configuration";
    return false;
  }
  try {
    ResolveOutputs();
    const std::size_t input_width = config_.input_width();
    if (config_.mode() == SHIELD) {
      ::sentil::Bounds bounds = config_.has_bounds() ? bounds_from_proto(config_.bounds())
                                                     : ::sentil::Bounds::unbounded(input_width);
      shield_ = std::make_unique<::sentil::SafetyFilter>(std::move(bounds));
      nominal_reader_ =
          node_->CreateReader<apollo::control::ControlCommand>(config_.nominal_channel());
    } else {
      ::sentil::SystemModel model = model_from_proto(config_.model(), input_width);
      ::sentil::Formula spec = formula_from_spec(config_.spec());
      const auto budget_ns = static_cast<std::uint64_t>(
          config_.deadline_fraction() * config_.control_period_ms() * 1e6);
      const std::size_t horizon = config_.model().horizon();
      ::sentil::Bounds bounds = config_.has_bounds()
                                    ? tile_bounds(config_.bounds(), input_width, horizon)
                                    : ::sentil::Bounds::unbounded(horizon * input_width);
      ::sentil::SmoothConfig smooth;
      const ::sentil::SmoothConfig* smooth_ptr = nullptr;
      if (config_.has_smooth()) {
        smooth.temperature = config_.smooth().temperature();
        smooth_ptr = &smooth;
      }
      controller_ = std::make_unique<::sentil::Controller>(
          std::move(model), std::move(spec), input_width, budget_ns, &bounds, smooth_ptr);
      for (const std::string& variable : config_.model().variables()) {
        variable_order_.push_back(variable);
      }
      for (const ChannelMapping& channel : config_.state_inputs()) {
        std::vector<FieldMapping> fields(channel.fields().begin(), channel.fields().end());
        extractor_.add_channel(channel.message_type(), fields);
      }
      localization_reader_ =
          node_->CreateReader<apollo::localization::LocalizationEstimate>("/apollo/localization/pose");
      chassis_reader_ = node_->CreateReader<apollo::canbus::Chassis>("/apollo/canbus/chassis");
    }
  } catch (const std::exception& error) {
    AERROR << "sentil control: configuration failed: " << error.what();
    return false;
  }
  const std::string channel =
      config_.mode() == ADVISORY ? std::string(kAdvisoryChannel) : config_.output_channel();
  control_writer_ = node_->CreateWriter<apollo::control::ControlCommand>(channel);
  AINFO << "sentil control: started in mode " << ControlMode_Name(config_.mode());
  return true;
}

void SentilControlComponent::ResolveOutputs() {
  const google::protobuf::Descriptor* command = apollo::control::ControlCommand::descriptor();
  for (const ControlOutput& output : config_.control_outputs()) {
    if (output.index() >= config_.input_width()) {
      throw std::invalid_argument("control_outputs index " + std::to_string(output.index()) +
                                  " is out of range for input_width " +
                                  std::to_string(config_.input_width()));
    }
    outputs_.emplace_back(static_cast<int>(output.index()),
                          resolve_actuation_field(command, output.field_path()));
  }
}

bool SentilControlComponent::BuildState(std::vector<double>* state) {
  std::vector<std::string> names;
  std::vector<double> values;
  localization_reader_->Observe();
  chassis_reader_->Observe();
  const auto localization = localization_reader_->GetLatestObserved();
  const auto chassis = chassis_reader_->GetLatestObserved();
  if (localization == nullptr || chassis == nullptr) {
    return false;
  }
  extractor_.extract_into(localization->GetTypeName(), *localization, &names, &values);
  extractor_.extract_into(chassis->GetTypeName(), *chassis, &names, &values);

  std::map<std::string, double> by_name;
  for (std::size_t i = 0; i < names.size(); ++i) {
    by_name[names[i]] = values[i];
  }
  state->clear();
  for (const std::string& variable : variable_order_) {
    const auto it = by_name.find(variable);
    if (it == by_name.end()) {
      return false;
    }
    state->push_back(it->second);
  }
  return true;
}

void SentilControlComponent::ApplyInputs(const std::vector<double>& input,
                                         apollo::control::ControlCommand* command) {
  for (const auto& output : outputs_) {
    if (output.first >= 0 && static_cast<std::size_t>(output.first) < input.size()) {
      set_actuation(command, output.second, input[output.first]);
    }
  }
}

void SentilControlComponent::Emit(const std::vector<double>& input) {
  apollo::control::ControlCommand command;
  ApplyInputs(input, &command);
  control_writer_->Write(command);
}

bool SentilControlComponent::Proc() {
  if (config_.mode() == SHIELD) {
    nominal_reader_->Observe();
    const auto nominal = nominal_reader_->GetLatestObserved();
    if (nominal == nullptr) {
      return true;
    }
    std::vector<double> command(config_.input_width(), 0.0);
    for (const auto& output : outputs_) {
      command[output.first] = get_actuation(*nominal, output.second);
    }
    try {
      const std::vector<double> safe = shield_->filter(command);
      apollo::control::ControlCommand out = *nominal;
      ApplyInputs(safe, &out);
      control_writer_->Write(out);
    } catch (const std::exception& error) {
      AERROR << "sentil control: shield failed: " << error.what();
      return false;
    }
    return true;
  }

  std::vector<double> state;
  if (!BuildState(&state)) {
    return true;
  }
  try {
    Emit(controller_->control(state));
  } catch (const std::exception& error) {
    AERROR << "sentil control: synthesis failed: " << error.what();
    return false;
  }
  return true;
}

}  // namespace sentil
}  // namespace apollo