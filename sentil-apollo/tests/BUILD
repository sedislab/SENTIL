load("//tools:apollo_package.bzl", "apollo_cc_test", "apollo_package")
load("//tools/proto:proto.bzl", "proto_library")

package(default_visibility = ["//visibility:public"])

proto_library(
    name = "test_messages_proto",
    srcs = ["test_messages.proto"],
)

apollo_cc_test(
    name = "field_extractor_test",
    size = "small",
    srcs = ["field_extractor_test.cc"],
    deps = [
        ":test_messages_cc_proto",
        "//modules/sentil/common:field_extractor",
        "@com_google_googletest//:gtest_main",
    ],
)

apollo_cc_test(
    name = "oracle_parity_test",
    size = "small",
    srcs = [
        "oracle_parity_test.cc",
        "json.hpp",
        "sentil_test.hpp",
    ],
    data = ["@sentil_cpp//:deterministic_oracle"],
    local_defines = ["SENTIL_ORACLE_PATH=\\\"../sentil_cpp/share/sentil/oracle.json\\\""],
    deps = [
        "//modules/sentil/common:engine_config",
        "//modules/sentil/proto:sentil_config_cc_proto",
        "//modules/sentil/proto:sentil_control_config_cc_proto",
        "@sentil_cpp//:sentil_cpp",
    ],
)

apollo_cc_test(
    name = "synthesizer_test",
    size = "small",
    srcs = [
        "sentil_test.hpp",
        "synthesizer_test.cc",
    ],
    deps = [
        "//modules/sentil/common:engine_config",
        "//modules/sentil/proto:sentil_control_config_cc_proto",
        "@sentil_cpp//:sentil_cpp",
    ],
)

apollo_cc_test(
    name = "example_synthesis_test",
    size = "small",
    srcs = [
        "example_synthesis_test.cc",
        "sentil_test.hpp",
    ],
    data = ["//modules/sentil/examples:configs"],
    local_defines = ["SENTIL_EXAMPLE_CONFIG=\\\"modules/sentil/examples/synthesize_control.pb.txt\\\""],
    deps = [
        "//modules/sentil/common:engine_config",
        "//modules/sentil/proto:sentil_control_config_cc_proto",
        "@sentil_cpp//:sentil_cpp",
    ],
)

apollo_cc_test(
    name = "synthesize_then_monitor_example_test",
    size = "small",
    srcs = ["//modules/sentil/examples:synthesize_then_monitor.cc"],
    deps = ["@sentil_cpp//:sentil_cpp"],
)

apollo_cc_test(
    name = "symbol_reference",
    size = "small",
    srcs = ["symbol_reference.cc"],
    deps = ["@sentil_cpp//:sentil_cpp"],
)

apollo_package()