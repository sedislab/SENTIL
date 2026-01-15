load("//tools:apollo_package.bzl", "apollo_component", "apollo_package")

package(default_visibility = ["//visibility:public"])

apollo_component(
    name = "libsentil_control_component.so",
    srcs = ["sentil_control_component.cc"],
    hdrs = ["sentil_control_component.h"],
    deps = [
        "//cyber",
        "//modules/common_msgs/chassis_msgs:chassis_cc_proto",
        "//modules/common_msgs/control_msgs:control_cmd_cc_proto",
        "//modules/common_msgs/localization_msgs:localization_cc_proto",
        "//modules/sentil/common:engine_config",
        "//modules/sentil/common:field_extractor",
        "//modules/sentil/proto:sentil_control_config_cc_proto",
        "@sentil_cpp//:sentil_cpp",
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