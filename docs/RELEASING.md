# Releasing SENTIL

This is the runbook for cutting a release and the map of where every artifact goes. The release model is deliberate by design: nothing reaches a registry on its own. A release happens when a maintainer pushes a version tag, and each upload runs only once the credential for that registry is configured, so the workflows can sit in the repository fully wired and still publish nothing until someone with the keys decides to.

## How a release fires

Push an annotated tag of the form `v1.0.0`. That tag is the signal every release workflow waits for. Each workflow builds and verifies its artifact on every run, but the upload step is guarded so it runs only on a tag push and only when its credential is present. Pushing a tag before a registry is set up still gives you a clean verification build with the artifact attached, and the upload simply does not happen.

Every workflow also has a manual `workflow_dispatch` trigger with a `dry_run` input that defaults to true, so you can exercise the packaging from the Actions tab without releasing anything.

Before the first publish to any registry, run `check-sentil-names.sh` to confirm the coordinate is still free, and claim it. These names are permanent once taken.

## What is wired now

Three distributors publish from this repository today, because their packages exist in the tree.

The engine goes to crates.io as the `sentil` crate, from `release-crates.yml`. It needs a `CARGO_REGISTRY_TOKEN` secret. The job runs `cargo publish --dry-run` first as the gate, then uploads on a tag push.

The Python package goes to PyPI as `sentil`, from `release-python.yml`. It builds abi3 wheels for Linux, macOS, and Windows on x86_64 and arm64, plus a source distribution. The extension is abi3 for Python 3.8 and up, so one wheel per platform serves every supported interpreter. Upload uses PyPI trusted publishing rather than a token: configure the trusted publisher on the PyPI project for this repository and the `pypi` environment, and put a required reviewer on that environment if you want a manual approval before any release goes out.

The C and C++ artifacts go to GitHub Releases, from `release-ffi.yml`. It bundles the shared and static libraries, the header, the pkg-config file, and the CMake package files into a per-platform tarball, builds the `.deb` and `.rpm`, and attaches them all to the release. This uses the automatic `GITHUB_TOKEN`, so it needs no setup.

## What lands with its binding

The remaining distributors are listed in the project layout, and each one's publish job ships with the binding it serves rather than as an empty shell now. A publish workflow that points at a directory that does not yet exist would fail on every run and would claim coverage the tree does not have, so these are recorded here as the plan and added when the binding is built.

| Distributor | Coordinate | Mechanism | Lands with |
| --- | --- | --- | --- |
| crates.io | `sentil` | `cargo publish` | wired |
| PyPI | `sentil` | maturin + trusted publishing | wired |
| GitHub Releases (C/C++) | tarball, `.deb`, `.rpm` | `gh-release` | wired |
| Maven Central | `io.github.sedislab:sentil` | Sonatype, Gradle publish | sentil-java |
| Julia General Registry | `Sentil` | Registrator and TagBot | sentil-jl |
| vcpkg and Conan | `sentil` | registry PR from the port files | sentil-cpp packaging |
| Homebrew, Scoop, Winget | CLI, Winget id `SEDIS.SENTIL` | tap, bucket, and manifest PRs | sentil-cli |
| MATLAB File Exchange | toolbox | linked GitHub release | sentil-matlab |
| ROS rosdistro | ROS 2 package | bloom release | sentil-ros |
| GitHub Releases (embedded) | every package and the raw archives | `release-embedded` on a tag | wired |
| Arduino Library Manager | library | one-time index PR, then tags auto-update | sentil-embedded |
| PlatformIO registry | `Sentil` | `pio pkg publish`, `PLATFORMIO_AUTH_TOKEN` secret | wired (secret-gated) |
| ESP component registry | `sedislab/sentil` | `compote component upload`, `IDF_COMPONENT_API_TOKEN` secret | wired (secret-gated) |

The C and C++ port files for vcpkg and Conan already live in `sentil-cpp`, so that row is packaging-complete; landing it is opening the registry PR, which is a maintainer step rather than an automated push.

## Cutting the release, in order

Bump the version in lockstep across every package and the `CITATION.cff`, and update the changelog. Let CI go green on the deterministic tier. Tag with `v` and the version, and push the tag. Watch the release workflows: each verifies, then uploads where its credential is set. For the registries that take a manual PR, the workflow stages the artifact and you open the PR. Confirm each package installs from its registry on a clean machine before announcing.