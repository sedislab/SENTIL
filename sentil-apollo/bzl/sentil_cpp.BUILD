package(default_visibility = ["//visibility:public"])

cc_library(
    name = "sentil_cpp",
    srcs = ["lib/libsentil.so"],
    hdrs = glob([
        "include/sentil/*.hpp",
        "include/sentil.h",
    ]),
    includes = ["include"],
)

filegroup(
    name = "deterministic_oracle",
    srcs = glob(["share/sentil/oracle.json"]),
)