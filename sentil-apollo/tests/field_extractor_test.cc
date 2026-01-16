#include "modules/sentil/common/field_extractor.h"

#include <cmath>
#include <string>
#include <vector>

#include "gtest/gtest.h"

#include "modules/sentil/proto/sentil_config.pb.h"
#include "modules/sentil/tests/test_messages.pb.h"

namespace apollo {
namespace sentil {
namespace {

FieldMapping path_mapping(const std::string& var, const std::string& path) {
  FieldMapping mapping;
  mapping.set_variable(var);
  mapping.set_field_path(path);
  return mapping;
}

FieldMapping builtin_mapping(const std::string& var, const std::string& builtin) {
  FieldMapping mapping;
  mapping.set_variable(var);
  mapping.set_builtin(builtin);
  return mapping;
}

TEST(FieldExtractor, ReadsANestedScalar) {
  test::Pose pose;
  pose.mutable_position()->set_x(7.5);
  pose.mutable_velocity()->set_y(-1.25);
  ResolvedField px(test::Pose::descriptor(), path_mapping("px", "position.x"));
  ResolvedField vy(test::Pose::descriptor(), path_mapping("vy", "velocity.y"));
  EXPECT_DOUBLE_EQ(px.extract(pose), 7.5);
  EXPECT_DOUBLE_EQ(vy.extract(pose), -1.25);
}

TEST(FieldExtractor, IndexesIntoTheObstacleList) {
  test::Perception perception;
  perception.add_perception_obstacle()->mutable_position()->set_x(3.0);
  perception.add_perception_obstacle()->mutable_position()->set_x(9.0);
  ResolvedField second(test::Perception::descriptor(),
                       path_mapping("d", "perception_obstacle[1].position.x"));
  EXPECT_DOUBLE_EQ(second.extract(perception), 9.0);
}

TEST(FieldExtractor, OutOfRangeIndexIsNaNNotZero) {
  test::Perception perception;
  ResolvedField first(test::Perception::descriptor(),
                      path_mapping("d", "perception_obstacle[0].position.x"));
  EXPECT_TRUE(std::isnan(first.extract(perception)));
}

TEST(FieldExtractor, NearestObstacleIsNotIndexZero) {
  test::Perception perception;
  auto* far = perception.add_perception_obstacle();
  far->mutable_position()->set_x(20.0);
  far->mutable_position()->set_y(0.0);
  auto* near = perception.add_perception_obstacle();
  near->mutable_position()->set_x(3.0);
  near->mutable_position()->set_y(4.0);
  ResolvedField nearest(test::Perception::descriptor(),
                        builtin_mapping("d", "NEAREST_OBSTACLE_DISTANCE"));
  EXPECT_DOUBLE_EQ(nearest.extract(perception), 5.0);
}

TEST(FieldExtractor, FrontGapIgnoresObstaclesBehindAndOutOfLane) {
  test::Perception perception;
  auto* behind = perception.add_perception_obstacle();
  behind->mutable_position()->set_x(-5.0);
  behind->mutable_position()->set_y(0.0);
  auto* aside = perception.add_perception_obstacle();
  aside->mutable_position()->set_x(8.0);
  aside->mutable_position()->set_y(5.0);
  auto* ahead = perception.add_perception_obstacle();
  ahead->mutable_position()->set_x(12.0);
  ahead->mutable_position()->set_y(0.5);
  ResolvedField gap(test::Perception::descriptor(), builtin_mapping("g", "FRONT_GAP"));
  EXPECT_DOUBLE_EQ(gap.extract(perception), 12.0);
}

TEST(FieldExtractor, MinTtcUsesClosingSpeed) {
  test::Perception perception;
  auto* obstacle = perception.add_perception_obstacle();
  obstacle->mutable_position()->set_x(10.0);
  obstacle->mutable_position()->set_y(0.0);
  obstacle->mutable_velocity()->set_x(-2.0);
  obstacle->mutable_velocity()->set_y(0.0);
  ResolvedField ttc(test::Perception::descriptor(), builtin_mapping("t", "MIN_TTC"));
  EXPECT_NEAR(ttc.extract(perception), 5.0, 1e-9);
}

TEST(FieldExtractor, EmptyListGivesInfiniteSafety) {
  test::Perception perception;
  ResolvedField nearest(test::Perception::descriptor(),
                        builtin_mapping("d", "NEAREST_OBSTACLE_DISTANCE"));
  EXPECT_TRUE(std::isinf(nearest.extract(perception)));
}

TEST(FieldExtractor, BadPathsFailAtResolution) {
  const auto* pose = test::Pose::descriptor();
  const auto* perception = test::Perception::descriptor();
  EXPECT_THROW(ResolvedField(pose, path_mapping("v", "position.nope")), FieldResolutionError);
  EXPECT_THROW(ResolvedField(pose, path_mapping("v", "nope.x")), FieldResolutionError);
  EXPECT_THROW(ResolvedField(pose, path_mapping("v", "position")), FieldResolutionError);
  EXPECT_THROW(ResolvedField(perception, path_mapping("v", "perception_obstacle.position.x")),
               FieldResolutionError);
  EXPECT_THROW(ResolvedField(pose, builtin_mapping("v", "NEAREST_OBSTACLE_DISTANCE")),
               FieldResolutionError);
  EXPECT_THROW(ResolvedField(perception, builtin_mapping("v", "WAT")), FieldResolutionError);
}

TEST(FieldExtractor, ChannelExtractsEveryMappedField) {
  test::Pose pose;
  pose.mutable_position()->set_x(1.0);
  pose.mutable_position()->set_y(2.0);
  FieldExtractor extractor;
  extractor.add_channel("apollo.sentil.test.Pose",
                        {path_mapping("px", "position.x"), path_mapping("py", "position.y")});
  std::vector<std::string> names;
  std::vector<double> values;
  extractor.extract_into("apollo.sentil.test.Pose", pose, &names, &values);
  ASSERT_EQ(names.size(), 2u);
  EXPECT_EQ(names[0], "px");
  EXPECT_DOUBLE_EQ(values[0], 1.0);
  EXPECT_EQ(names[1], "py");
  EXPECT_DOUBLE_EQ(values[1], 2.0);
}

}  // namespace
}  // namespace sentil
}  // namespace apollo