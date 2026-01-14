#include "modules/sentil/common/field_extractor.h"

#include <algorithm>
#include <cmath>
#include <limits>

namespace apollo {
namespace sentil {
namespace {

using google::protobuf::Descriptor;
using google::protobuf::FieldDescriptor;
using google::protobuf::Message;
using google::protobuf::Reflection;

// Half the ego lane corridor in meters
constexpr double kLaneHalfWidth = 1.75;

constexpr char kObstacleList[] = "perception_obstacle";
constexpr char kPosition[] = "position";
constexpr char kVelocity[] = "velocity";

struct Segment {
  std::string name;
  int index;  // -1 when the segment has no [index]
};

Segment parse_segment(const std::string& token) {
  const auto open = token.find('[');
  if (open == std::string::npos) {
    return {token, -1};
  }
  const auto close = token.find(']', open);
  if (close == std::string::npos) {
    throw FieldResolutionError("malformed array index in '" + token + "'");
  }
  const std::string digits = token.substr(open + 1, close - open - 1);
  try {
    return {token.substr(0, open), std::stoi(digits)};
  } catch (const std::exception&) {
    throw FieldResolutionError("'" + digits + "' is not an array index in '" + token + "'");
  }
}

std::vector<Segment> parse_path(const std::string& path) {
  std::vector<Segment> out;
  std::size_t start = 0;
  while (true) {
    const auto dot = path.find('.', start);
    const std::string token =
        path.substr(start, dot == std::string::npos ? std::string::npos : dot - start);
    if (token.empty()) {
      throw FieldResolutionError("empty segment in field path '" + path + "'");
    }
    out.push_back(parse_segment(token));
    if (dot == std::string::npos) {
      break;
    }
    start = dot + 1;
  }
  return out;
}

bool is_numeric(const FieldDescriptor* field) {
  switch (field->cpp_type()) {
    case FieldDescriptor::CPPTYPE_DOUBLE:
    case FieldDescriptor::CPPTYPE_FLOAT:
    case FieldDescriptor::CPPTYPE_INT32:
    case FieldDescriptor::CPPTYPE_INT64:
    case FieldDescriptor::CPPTYPE_UINT32:
    case FieldDescriptor::CPPTYPE_UINT64:
    case FieldDescriptor::CPPTYPE_BOOL:
    case FieldDescriptor::CPPTYPE_ENUM:
      return true;
    default:
      return false;
  }
}

double read_singular(const Message& message, const FieldDescriptor* field) {
  const Reflection* r = message.GetReflection();
  switch (field->cpp_type()) {
    case FieldDescriptor::CPPTYPE_DOUBLE: return r->GetDouble(message, field);
    case FieldDescriptor::CPPTYPE_FLOAT:  return r->GetFloat(message, field);
    case FieldDescriptor::CPPTYPE_INT32:  return r->GetInt32(message, field);
    case FieldDescriptor::CPPTYPE_INT64:  return static_cast<double>(r->GetInt64(message, field));
    case FieldDescriptor::CPPTYPE_UINT32: return r->GetUInt32(message, field);
    case FieldDescriptor::CPPTYPE_UINT64: return static_cast<double>(r->GetUInt64(message, field));
    case FieldDescriptor::CPPTYPE_BOOL:   return r->GetBool(message, field) ? 1.0 : 0.0;
    case FieldDescriptor::CPPTYPE_ENUM:   return r->GetEnumValue(message, field);
    default:
      break;
  }
  throw FieldResolutionError("field '" + std::string(field->name()) + "' is not numeric");
}

double read_repeated(const Message& message, const FieldDescriptor* field, int index) {
  const Reflection* r = message.GetReflection();
  switch (field->cpp_type()) {
    case FieldDescriptor::CPPTYPE_DOUBLE: return r->GetRepeatedDouble(message, field, index);
    case FieldDescriptor::CPPTYPE_FLOAT:  return r->GetRepeatedFloat(message, field, index);
    case FieldDescriptor::CPPTYPE_INT32:  return r->GetRepeatedInt32(message, field, index);
    case FieldDescriptor::CPPTYPE_INT64:  return static_cast<double>(r->GetRepeatedInt64(message, field, index));
    case FieldDescriptor::CPPTYPE_UINT32: return r->GetRepeatedUInt32(message, field, index);
    case FieldDescriptor::CPPTYPE_UINT64: return static_cast<double>(r->GetRepeatedUInt64(message, field, index));
    case FieldDescriptor::CPPTYPE_BOOL:   return r->GetRepeatedBool(message, field, index) ? 1.0 : 0.0;
    case FieldDescriptor::CPPTYPE_ENUM:   return r->GetRepeatedEnumValue(message, field, index);
    default:
      break;
  }
  throw FieldResolutionError("field '" + std::string(field->name()) + "' is not numeric");
}

const FieldDescriptor* require_field(const Descriptor* type, const std::string& name,
                                     const std::string& context) {
  const FieldDescriptor* field = type->FindFieldByName(name);
  if (field == nullptr) {
    throw FieldResolutionError(context + ": '" + std::string(type->full_name()) +
                               "' has no field '" + name + "'");
  }
  return field;
}

const FieldDescriptor* require_scalar(const Descriptor* type, const std::string& name,
                                      const std::string& context) {
  const FieldDescriptor* field = require_field(type, name, context);
  if (field->is_repeated() || !is_numeric(field)) {
    throw FieldResolutionError(context + ": '" + name + "' is not a numeric scalar");
  }
  return field;
}

}  // namespace

ResolvedField::ResolvedField(const Descriptor* root, const FieldMapping& mapping) {
  variable_ = mapping.variable();
  if (variable_.empty()) {
    throw FieldResolutionError("a field mapping has no variable name");
  }
  if (mapping.has_builtin()) {
    is_builtin_ = true;
    resolve_builtin(root, mapping.builtin());
  } else if (mapping.has_field_path()) {
    resolve_path(root, mapping.field_path());
  } else {
    throw FieldResolutionError("variable '" + variable_ + "' has neither a field_path nor a builtin");
  }
}

void ResolvedField::resolve_path(const Descriptor* root, const std::string& path) {
  const std::vector<Segment> segments = parse_path(path);
  const Descriptor* type = root;
  for (std::size_t i = 0; i < segments.size(); ++i) {
    const Segment& seg = segments[i];
    const std::string ctx = "field path '" + path + "'";
    const FieldDescriptor* field = require_field(type, seg.name, ctx);
    if (seg.index >= 0 && !field->is_repeated()) {
      throw FieldResolutionError(ctx + ": '" + seg.name + "' is not a repeated field");
    }
    if (seg.index < 0 && field->is_repeated()) {
      throw FieldResolutionError(ctx + ": repeated field '" + seg.name + "' needs an index");
    }
    const bool last = i + 1 == segments.size();
    if (last) {
      if (field->cpp_type() == FieldDescriptor::CPPTYPE_MESSAGE || !is_numeric(field)) {
        throw FieldResolutionError(ctx + ": '" + seg.name + "' is not a numeric scalar");
      }
    } else if (field->cpp_type() != FieldDescriptor::CPPTYPE_MESSAGE) {
      throw FieldResolutionError(ctx + ": cannot descend into non-message field '" + seg.name + "'");
    } else {
      type = field->message_type();
    }
    path_.push_back({field, seg.index});
  }
}

void ResolvedField::resolve_builtin(const Descriptor* root, const std::string& name) {
  if (name == "NEAREST_OBSTACLE_DISTANCE") {
    builtin_ = Builtin::kNearestObstacleDistance;
  } else if (name == "MIN_TTC") {
    builtin_ = Builtin::kMinTimeToCollision;
  } else if (name == "FRONT_GAP") {
    builtin_ = Builtin::kFrontGap;
  } else {
    throw FieldResolutionError("unknown builtin '" + name + "' for variable '" + variable_ +
                               "'; expected NEAREST_OBSTACLE_DISTANCE, MIN_TTC, or FRONT_GAP");
  }
  const std::string ctx = "builtin " + name;
  obstacle_.list = require_field(root, kObstacleList, ctx);
  if (!obstacle_.list->is_repeated() ||
      obstacle_.list->cpp_type() != FieldDescriptor::CPPTYPE_MESSAGE) {
    throw FieldResolutionError(ctx + ": '" + std::string(kObstacleList) + "' is not a repeated message");
  }
  const Descriptor* obstacle = obstacle_.list->message_type();
  obstacle_.pos = require_field(obstacle, kPosition, ctx);
  if (obstacle_.pos->cpp_type() != FieldDescriptor::CPPTYPE_MESSAGE) {
    throw FieldResolutionError(ctx + ": '" + std::string(kPosition) + "' is not a message");
  }
  obstacle_.pos_x = require_scalar(obstacle_.pos->message_type(), "x", ctx);
  obstacle_.pos_y = require_scalar(obstacle_.pos->message_type(), "y", ctx);
  if (builtin_ == Builtin::kMinTimeToCollision) {
    obstacle_.vel = require_field(obstacle, kVelocity, ctx);
    if (obstacle_.vel->cpp_type() != FieldDescriptor::CPPTYPE_MESSAGE) {
      throw FieldResolutionError(ctx + ": '" + std::string(kVelocity) + "' is not a message");
    }
    obstacle_.vel_x = require_scalar(obstacle_.vel->message_type(), "x", ctx);
    obstacle_.vel_y = require_scalar(obstacle_.vel->message_type(), "y", ctx);
  }
}

double ResolvedField::extract(const Message& message) const {
  return is_builtin_ ? evaluate_builtin(message) : walk_path(message);
}

double ResolvedField::walk_path(const Message& message) const {
  const Message* current = &message;
  for (std::size_t i = 0; i < path_.size(); ++i) {
    const Step& step = path_[i];
    const Reflection* r = current->GetReflection();
    const bool last = i + 1 == path_.size();
    if (step.index >= 0) {
      if (step.index >= r->FieldSize(*current, step.field)) {
        return std::numeric_limits<double>::quiet_NaN();
      }
    }
    if (last) {
      return step.index >= 0 ? read_repeated(*current, step.field, step.index)
                             : read_singular(*current, step.field);
    }
    current = step.index >= 0 ? &r->GetRepeatedMessage(*current, step.field, step.index)
                              : &r->GetMessage(*current, step.field);
  }
  return std::numeric_limits<double>::quiet_NaN();
}

double ResolvedField::evaluate_builtin(const Message& message) const {
  const Reflection* r = message.GetReflection();
  const int count = r->FieldSize(message, obstacle_.list);
  double best = std::numeric_limits<double>::infinity();
  for (int i = 0; i < count; ++i) {
    const Message& obstacle = r->GetRepeatedMessage(message, obstacle_.list, i);
    const Reflection* obstacle_r = obstacle.GetReflection();
    const Message& position = obstacle_r->GetMessage(obstacle, obstacle_.pos);
    const double x = read_singular(position, obstacle_.pos_x);
    const double y = read_singular(position, obstacle_.pos_y);
    switch (builtin_) {
      case Builtin::kNearestObstacleDistance:
        best = std::min(best, std::hypot(x, y));
        break;
      case Builtin::kFrontGap:
        if (x > 0.0 && std::abs(y) <= kLaneHalfWidth) {
          best = std::min(best, x);
        }
        break;
      case Builtin::kMinTimeToCollision: {
        const Message& velocity = obstacle_r->GetMessage(obstacle, obstacle_.vel);
        const double vx = read_singular(velocity, obstacle_.vel_x);
        const double vy = read_singular(velocity, obstacle_.vel_y);
        const double range = std::hypot(x, y);
        const double closing = -(x * vx + y * vy) / std::max(range, 1e-6);
        if (closing > 1e-3) {
          best = std::min(best, range / closing);
        }
        break;
      }
    }
  }
  return best;
}

void FieldExtractor::add_channel(const std::string& message_type,
                                 const std::vector<FieldMapping>& fields) {
  const Descriptor* type =
      google::protobuf::DescriptorPool::generated_pool()->FindMessageTypeByName(message_type);
  if (type == nullptr) {
    throw FieldResolutionError("message type '" + message_type + "' is not in the descriptor pool");
  }
  std::vector<ResolvedField>& resolved = by_type_[message_type];
  for (const FieldMapping& field : fields) {
    resolved.emplace_back(type, field);
  }
}

void FieldExtractor::extract_into(const std::string& message_type, const Message& message,
                                  std::vector<std::string>* names,
                                  std::vector<double>* values) const {
  const auto it = by_type_.find(message_type);
  if (it == by_type_.end()) {
    return;
  }
  for (const ResolvedField& field : it->second) {
    const double value = field.extract(message);
    if (std::isnan(value)) {
      continue;
    }
    names->push_back(field.variable());
    values->push_back(value);
  }
}

}  // namespace sentil
}  // namespace apollo