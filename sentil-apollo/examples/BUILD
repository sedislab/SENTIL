load("//tools:apollo_package.bzl", "apollo_cc_binary", "apollo_component", "apollo_package")

package(default_visibility = ["//visibility:public"])

apollo_component(
    name = "libsafety_planner_subscriber.so",
    srcs = ["safety_planner_subscriber.cc"],
    deps = [
        "//cyber",
        "//modules/sentil/proto:sentil_status_cc_proto",
    ],
)

apollo_cc_binary(
    name = "synthesize_then_monitor",
    srcs = ["synthesize_then_monitor.cc"],
    deps = ["@sentil_cpp//:sentil_cpp"],
)

filegroup(
    name = "configs",
    srcs = glob(["*.pb.txt"]),
)

exports_files(["synthesize_then_monitor.cc"])

apollo_package()