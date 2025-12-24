#include <gtest/gtest.h>

#include <geometry_msgs/msg/pose.hpp>
#include <geometry_msgs/msg/twist.hpp>
#include <rosidl_typesupport_introspection_cpp/message_type_support_decl.hpp>
#include <sensor_msgs/msg/laser_scan.hpp>

#include "sentil_ros/field_extractor.hpp"

using sentil_ros::FieldExtractorError;
using sentil_ros::introspection::extract_double_from_field;

template<typename T>
const rosidl_message_type_support_t * introspection_handle()
{
  return rosidl_typesupport_introspection_cpp::get_message_type_support_handle<T>();
}

TEST(FieldExtractor, ReadsANestedScalar)
{
  geometry_msgs::msg::Twist twist;
  twist.linear.x = 1.23;
  twist.angular.z = 4.56;
  const auto * ts = introspection_handle<geometry_msgs::msg::Twist>();
  EXPECT_NEAR(extract_double_from_field(&twist, ts, "linear.x"), 1.23, 1e-9);
  EXPECT_NEAR(extract_double_from_field(&twist, ts, "angular.z"), 4.56, 1e-9);
}

TEST(FieldExtractor, ReadsThroughTwoMessageLevels)
{
  geometry_msgs::msg::Pose pose;
  pose.position.x = 7.89;
  pose.orientation.w = 1.0;
  const auto * ts = introspection_handle<geometry_msgs::msg::Pose>();
  EXPECT_NEAR(extract_double_from_field(&pose, ts, "position.x"), 7.89, 1e-9);
  EXPECT_NEAR(extract_double_from_field(&pose, ts, "orientation.w"), 1.0, 1e-9);
}

TEST(FieldExtractor, IndexesIntoASequence)
{
  sensor_msgs::msg::LaserScan scan;
  scan.ranges = {1.0F, 2.5F, 3.0F};
  const auto * ts = introspection_handle<sensor_msgs::msg::LaserScan>();
  EXPECT_NEAR(extract_double_from_field(&scan, ts, "ranges[1]"), 2.5, 1e-6);
  EXPECT_THROW(extract_double_from_field(&scan, ts, "ranges[100]"), FieldExtractorError);
}

TEST(FieldExtractor, RejectsBadPaths)
{
  geometry_msgs::msg::Twist twist;
  const auto * ts = introspection_handle<geometry_msgs::msg::Twist>();
  EXPECT_THROW(extract_double_from_field(&twist, ts, "linear.nope"), FieldExtractorError);
  EXPECT_THROW(extract_double_from_field(&twist, ts, "nope.x"), FieldExtractorError);
  EXPECT_THROW(extract_double_from_field(&twist, ts, "linear"), FieldExtractorError);
  EXPECT_THROW(extract_double_from_field(&twist, ts, "linear.x[0]"), FieldExtractorError);
}

int main(int argc, char ** argv)
{
  ::testing::InitGoogleTest(&argc, argv);
  return RUN_ALL_TESTS();
}