load("//tools:apollo_package.bzl", "apollo_package")
load("//tools/proto:proto.bzl", "proto_library", "cc_proto_library")

package(default_visibility = ["//visibility:public"])

proto_library(
    name = "sentil_config_proto",
    srcs = ["sentil_config.proto"],
)

cc_proto_library(
    name = "sentil_config_cc_proto",
    deps = [":sentil_config_proto"],
)

proto_library(
    name = "sentil_status_proto",
    srcs = ["sentil_status.proto"],
    deps = ["//modules/common_msgs/basic_msgs:header_proto"],
)

cc_proto_library(
    name = "sentil_status_cc_proto",
    deps = [":sentil_status_proto"],
)

apollo_package()