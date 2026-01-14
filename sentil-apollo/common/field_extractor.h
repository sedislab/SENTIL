#pragma once

#include <map>
#include <stdexcept>
#include <string>
#include <vector>

#include "google/protobuf/descriptor.h"
#include "google/protobuf/message.h"

#include "modules/sentil/proto/sentil_config.pb.h"

namespace apollo {
namespace sentil {

class FieldResolutionError : public std::runtime_error {
 public:
  explicit FieldResolutionError(const std::string& message) : std::runtime_error(message) {}
};

/// Reductions over the perception obstacle list, in the ego frame.
enum class Builtin {
  kNearestObstacleDistance,
  kMinTimeToCollision,
  kFrontGap,
};

class ResolvedField {
 public:
  ResolvedField(const google::protobuf::Descriptor* root, const FieldMapping& mapping);

  const std::string& variable() const { return variable_; }

  double extract(const google::protobuf::Message& message) const;

 private:
  struct Step {
    const google::protobuf::FieldDescriptor* field;
    int index;  // -1 for a singular field
  };

  struct ObstacleFields {
    const google::protobuf::FieldDescriptor* list;
    const google::protobuf::FieldDescriptor* pos;
    const google::protobuf::FieldDescriptor* pos_x;
    const google::protobuf::FieldDescriptor* pos_y;
    const google::protobuf::FieldDescriptor* vel;
    const google::protobuf::FieldDescriptor* vel_x;
    const google::protobuf::FieldDescriptor* vel_y;
  };

  void resolve_path(const google::protobuf::Descriptor* root, const std::string& path);
  void resolve_builtin(const google::protobuf::Descriptor* root, const std::string& name);
  double walk_path(const google::protobuf::Message& message) const;
  double evaluate_builtin(const google::protobuf::Message& message) const;

  std::string variable_;
  bool is_builtin_ = false;
  Builtin builtin_{};
  ObstacleFields obstacle_{};
  std::vector<Step> path_;
};

class FieldExtractor {
 public:
  void add_channel(const std::string& message_type, const std::vector<FieldMapping>& fields);

  void extract_into(const std::string& message_type, const google::protobuf::Message& message,
                    std::vector<std::string>* names, std::vector<double>* values) const;

 private:
  std::map<std::string, std::vector<ResolvedField>> by_type_;
};

}  // namespace sentil
}  // namespace apollo