#include "sentil_ros/field_extractor.hpp"

#include <cstdint>
#include <sstream>
#include <string>
#include <vector>

#include "rosidl_typesupport_introspection_cpp/field_types.hpp"
#include "rosidl_typesupport_introspection_cpp/identifier.hpp"
#include "rosidl_typesupport_introspection_cpp/message_introspection.hpp"

namespace sentil_ros
{
namespace introspection
{
namespace
{

namespace intro = rosidl_typesupport_introspection_cpp;
using Members = intro::MessageMembers;
using Member = intro::MessageMember;

struct Token
{
  std::string name;
  // -1 when the token carries no `[index]`.
  long index;
};

Token parse_token(const std::string & token)
{
  const auto open = token.find('[');
  if (open == std::string::npos) {
    return {token, -1};
  }
  const auto close = token.find(']', open);
  if (close == std::string::npos || close < open) {
    throw FieldExtractorError("malformed array index in '" + token + "'");
  }
  const std::string digits = token.substr(open + 1, close - open - 1);
  try {
    return {token.substr(0, open), std::stol(digits)};
  } catch (const std::exception &) {
    throw FieldExtractorError("'" + digits + "' is not an array index in '" + token + "'");
  }
}

const Members * as_members(const rosidl_message_type_support_t * type_support)
{
  if (type_support == nullptr) {
    throw FieldExtractorError("null type support handle");
  }
  if (std::string(type_support->typesupport_identifier) != intro::typesupport_identifier) {
    throw FieldExtractorError(
      std::string("expected introspection type support, got '") +
      type_support->typesupport_identifier + "'");
  }
  const auto * members = static_cast<const Members *>(type_support->data);
  if (members == nullptr) {
    throw FieldExtractorError("null message members handle");
  }
  return members;
}

const Member * find_member(const Members * members, const std::string & name)
{
  for (uint32_t i = 0; i < members->member_count_; ++i) {
    if (name == members->members_[i].name_) {
      return &members->members_[i];
    }
  }
  throw FieldExtractorError("field not found: " + name);
}

double read_scalar(const void * data, const Member * member)
{
  const auto * bytes = static_cast<const unsigned char *>(data);
  switch (member->type_id_) {
    case intro::ROS_TYPE_DOUBLE:
      return *reinterpret_cast<const double *>(bytes);
    case intro::ROS_TYPE_FLOAT:
      return static_cast<double>(*reinterpret_cast<const float *>(bytes));
    case intro::ROS_TYPE_INT64:
      return static_cast<double>(*reinterpret_cast<const int64_t *>(bytes));
    case intro::ROS_TYPE_INT32:
      return static_cast<double>(*reinterpret_cast<const int32_t *>(bytes));
    case intro::ROS_TYPE_INT16:
      return static_cast<double>(*reinterpret_cast<const int16_t *>(bytes));
    case intro::ROS_TYPE_INT8:
      return static_cast<double>(*reinterpret_cast<const int8_t *>(bytes));
    case intro::ROS_TYPE_UINT64:
      return static_cast<double>(*reinterpret_cast<const uint64_t *>(bytes));
    case intro::ROS_TYPE_UINT32:
      return static_cast<double>(*reinterpret_cast<const uint32_t *>(bytes));
    case intro::ROS_TYPE_UINT16:
      return static_cast<double>(*reinterpret_cast<const uint16_t *>(bytes));
    case intro::ROS_TYPE_UINT8:
      return static_cast<double>(*reinterpret_cast<const uint8_t *>(bytes));
    default:
      throw FieldExtractorError(
        "field is not numeric (introspection type " + std::to_string(member->type_id_) + ")");
  }
}

double walk(
  const void * data, const rosidl_message_type_support_t * type_support,
  const std::vector<Token> & tokens, size_t depth)
{
  const Members * members = as_members(type_support);
  const Token & token = tokens.at(depth);
  const Member * member = find_member(members, token.name);

  if (token.index >= 0 && !member->is_array_) {
    throw FieldExtractorError("cannot index into non-array field: " + token.name);
  }
  if (token.index < 0 && member->is_array_) {
    throw FieldExtractorError("array field needs an index: " + token.name);
  }

  const auto * base = static_cast<const unsigned char *>(data);
  const void * element;
  if (member->is_array_) {
    const void * array = base + member->offset_;
    size_t size = (member->array_size_ > 0 && !member->is_upper_bound_)
      ? member->array_size_
      : (member->size_function ? member->size_function(array) : 0);
    if (static_cast<size_t>(token.index) >= size) {
      throw FieldExtractorError(
        "index " + std::to_string(token.index) + " out of bounds for " + token.name +
        " (size " + std::to_string(size) + ")");
    }
    if (member->get_const_function == nullptr) {
      throw FieldExtractorError("introspection has no element accessor for " + token.name);
    }
    element = member->get_const_function(array, static_cast<size_t>(token.index));
  } else {
    element = base + member->offset_;
  }

  const bool last = depth + 1 == tokens.size();
  if (last) {
    if (member->type_id_ == intro::ROS_TYPE_MESSAGE) {
      throw FieldExtractorError("'" + token.name + "' is a message, not a scalar");
    }
    return read_scalar(element, member);
  }
  if (member->type_id_ != intro::ROS_TYPE_MESSAGE) {
    throw FieldExtractorError("cannot descend into non-message field: " + token.name);
  }
  const auto * nested = static_cast<const rosidl_message_type_support_t *>(member->members_);
  return walk(element, nested, tokens, depth + 1);
}

}  // namespace

double extract_double_from_field(
  const void * msg_data, const rosidl_message_type_support_t * type_support,
  const std::string & field_name)
{
  if (msg_data == nullptr) {
    throw FieldExtractorError("null message data");
  }
  std::vector<Token> tokens;
  std::stringstream stream(field_name);
  std::string segment;
  while (std::getline(stream, segment, '.')) {
    if (segment.empty()) {
      throw FieldExtractorError("empty segment in field path '" + field_name + "'");
    }
    tokens.push_back(parse_token(segment));
  }
  if (tokens.empty()) {
    throw FieldExtractorError("empty field path");
  }
  return walk(msg_data, type_support, tokens, 0);
}

}  // namespace introspection

}  // namespace sentil_ros