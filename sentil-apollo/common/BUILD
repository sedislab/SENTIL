load("//tools:apollo_package.bzl", "apollo_cc_library", "apollo_package")

package(default_visibility = ["//visibility:public"])

apollo_cc_library(
    name = "field_extractor",
    srcs = ["field_extractor.cc"],
    hdrs = ["field_extractor.h"],
    deps = [
        "//modules/sentil/proto:sentil_config_cc_proto",
        "@com_google_protobuf//:protobuf",
    ],
)

apollo_cc_library(
    name = "engine_config",
    srcs = ["engine_config.cc"],
    hdrs = ["engine_config.h"],
    deps = [
        "//modules/sentil/proto:sentil_config_cc_proto",
        "//modules/sentil/proto:sentil_control_config_cc_proto",
        "@sentil_cpp//:sentil_cpp",
    ],
)

apollo_package()