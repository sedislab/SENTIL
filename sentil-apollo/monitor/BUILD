load("//tools:apollo_package.bzl", "apollo_cc_library", "apollo_component", "apollo_package")

package(default_visibility = ["//visibility:public"])

MSG_DEPS = [
    "//modules/common_msgs/chassis_msgs:chassis_cc_proto",
    "//modules/common_msgs/localization_msgs:localization_cc_proto",
    "//modules/common_msgs/perception_msgs:perception_obstacle_cc_proto",
]

apollo_cc_library(
    name = "monitor_engine",
    srcs = ["monitor_engine.cc"],
    hdrs = ["monitor_engine.h"],
    deps = MSG_DEPS + [
        "//cyber",
        "//modules/sentil/common:engine_config",
        "//modules/sentil/common:field_extractor",
        "//modules/sentil/proto:sentil_config_cc_proto",
        "//modules/sentil/proto:sentil_status_cc_proto",
        "@sentil_cpp//:sentil_cpp",
    ],
)

apollo_component(
    name = "libsentil_monitor_component.so",
    srcs = ["sentil_monitor_component.cc"],
    hdrs = ["sentil_monitor_component.h"],
    deps = MSG_DEPS + [
        ":monitor_engine",
        "//cyber",
        "//modules/sentil/proto:sentil_config_cc_proto",
        "//modules/sentil/proto:sentil_status_cc_proto",
    ],
)

apollo_component(
    name = "libsentil_timed_monitor_component.so",
    srcs = ["sentil_timed_monitor_component.cc"],
    hdrs = ["sentil_timed_monitor_component.h"],
    deps = MSG_DEPS + [
        ":monitor_engine",
        "//cyber",
        "//modules/sentil/proto:sentil_config_cc_proto",
        "//modules/sentil/proto:sentil_status_cc_proto",
    ],
)

filegroup(
    name = "runtime_files",
    srcs = glob([
        "conf/**",
        "dag/**",
        "launch/**",
    ]),
)

apollo_package()