# Maintainer notes

The workflows here build, test, and release SENTIL to the distributors. The CI runs the part of the deterministic tests and benchmarks. It leaves the `sentil-benchmarks` tests typechecked but unrun, so `make verify` or the CPU Docker stage still has to pass locally before a tag. It only uploads the packages to the distributors on a tag push.

[docs/RELEASING.md](../docs/RELEASING.md) contains more release details.