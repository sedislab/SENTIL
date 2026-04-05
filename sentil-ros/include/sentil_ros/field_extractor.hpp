#ifndef SENTIL_ROS_FIELD_EXTRACTOR_HPP
#define SENTIL_ROS_FIELD_EXTRACTOR_HPP

#include <optional>
#include <stdexcept>
#include <string>

#include <rosidl_runtime_c/message_type_support_struct.h>

namespace sentil_ros
{

/// Raised when a path does not resolve to a numeric scalar.
class FieldExtractorError : public std::runtime_error
{
public:
  explicit FieldExtractorError(const std::string & message) : std::runtime_error(message) {}
};

namespace introspection
{

/// Reads a dotted path such as `pose.position.x` or `ranges[1]` as a double.
double extract_double_from_field(
  const void * msg_data,
  const rosidl_message_type_support_t * type_support,
  const std::string & field_name);

/// Reads `header.stamp` as seconds.
std::optional<double> extract_header_stamp(
  const void * msg_data,
  const rosidl_message_type_support_t * type_support);

}  // namespace introspection

}  // namespace sentil_ros

#endif  // SENTIL_ROS_FIELD_EXTRACTOR_HPP