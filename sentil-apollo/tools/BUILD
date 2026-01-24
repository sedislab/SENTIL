load("//tools:apollo_package.bzl", "apollo_cc_binary", "apollo_package")

package(default_visibility = ["//visibility:public"])

apollo_cc_binary(
    name = "sentil_record_analyzer",
    srcs = ["sentil_record_analyzer.cc"],
    deps = [
        "//cyber",
        "//cyber/record:cyber_record",
        "//modules/sentil/common:engine_config",
        "//modules/sentil/common:field_extractor",
        "//modules/sentil/proto:sentil_config_cc_proto",
        "@com_github_gflags_gflags//:gflags",
        "@sentil_cpp//:sentil_cpp",
    ],
)

apollo_cc_binary(
    name = "sentil_synthesizer",
    srcs = ["sentil_synthesizer.cc"],
    deps = [
        "//modules/sentil/common:engine_config",
        "//modules/sentil/proto:sentil_control_config_cc_proto",
        "@com_github_gflags_gflags//:gflags",
        "@sentil_cpp//:sentil_cpp",
    ],
)

apollo_package()