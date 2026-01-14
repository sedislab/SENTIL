# BUILD for the @sentil_cpp external repository. It wraps the prebuilt SENTIL C++ surface
# and the bundled core so the Apollo module links the engine without a Rust toolchain.
# Point the repository at a SENTIL install tree (see the README for the WORKSPACE or
# MODULE.bazel snippet); the tree holds include/sentil/*.hpp, include/sentil.h, and
# lib/libsentil.so.
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