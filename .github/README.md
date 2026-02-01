# Maintainer notes

The workflows here build, test, and release SENTIL. Continuous integration runs the deterministic, hardware-independent tier on every push, and the GPU jobs skip cleanly when no device is present. Each release workflow builds and verifies its distributor's artifact on every run and uploads it only on a tag push when its credential is set, so a tag pushed before a registry is configured still produces a clean verification build.

The release runbook, including every secret and variable each workflow needs and how to obtain it, is in [docs/RELEASING.md](../docs/RELEASING.md).