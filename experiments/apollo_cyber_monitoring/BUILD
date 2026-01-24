load("//tools:apollo_package.bzl", "apollo_cc_binary", "apollo_package")

package(default_visibility = ["//visibility:public"])

apollo_cc_binary(
    name = "scenario_publisher",
    srcs = ["scenario_publisher.cc"],
    deps = [
        "//cyber",
        "//modules/common_msgs/chassis_msgs:chassis_cc_proto",
        "//modules/common_msgs/localization_msgs:localization_cc_proto",
        "//modules/common_msgs/perception_msgs:perception_obstacle_cc_proto",
    ],
)

apollo_cc_binary(
    name = "verdict_echo",
    srcs = ["verdict_echo.cc"],
    deps = [
        "//cyber",
        "//modules/sentil/proto:sentil_status_cc_proto",
    ],
)

apollo_package()