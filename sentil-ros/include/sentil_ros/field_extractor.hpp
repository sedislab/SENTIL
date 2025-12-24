#ifndef SENTIL_ROS_FIELD_EXTRACTOR_HPP
#define SENTIL_ROS_FIELD_EXTRACTOR_HPP

#include <stdexcept>
#include <string>

#include <rclcpp/time.hpp>
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

}  // namespace introspection

/// Reads `header.stamp` from a message that carries a `std_msgs/Header`, returning
/// it as an `rclcpp::Time`. Throws if the message has no header.
rclcpp::Time extract_header_time(
  const void * msg_data, const rosidl_message_type_support_t * type_support);

}  // namespace sentil_ros

#endif  // SENTIL_ROS_FIELD_EXTRACTOR_HPP