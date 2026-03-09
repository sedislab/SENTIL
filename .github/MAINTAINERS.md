# Maintainer notes

The workflows here build, test, and release SENTIL. CI runs the deterministic, hardware-independent tier on every push, and the GPU jobs skip when no device is present. Each release workflow builds and verifies its distributor's artifact on every run and uploads it only on a tag push.

[docs/RELEASING.md](../docs/RELEASING.md) contains more release details.