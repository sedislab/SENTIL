# Packaging the sentil CLI

This directory holds the distributor manifests for the `sentil` command-line tool. The `release-cli` workflow builds an archive for each target, runs `stage.sh` to bundle the binary with its completions, man page, and licenses, and runs `fill-manifests.sh` to stamp the manifests with the version and each archive's checksum. The manifests checked in here carry placeholder checksums, so the repository never holds a hash that a later rebuild would invalidate.

## What is here

- `homebrew/sentil.rb` is the Homebrew formula. It carries a download URL and SHA-256 for each macOS build, Apple silicon and Intel, and for x86_64 Linux, and installs `sentil` along with its man page and shell completions.
- `scoop/sentil.json` is the Scoop manifest for Windows x64: the archive URL and hash, and the `bin` path to `sentil.exe` inside the unpacked archive.
- `winget/SEDIS.SENTIL.yaml`, `winget/SEDIS.SENTIL.installer.yaml`, and `winget/SEDIS.SENTIL.locale.en-US.yaml` are the version, installer, and locale manifests that Winget requires for the package id `SEDIS.SENTIL`.
- `stage.sh <triple> <version> <bin-path> [tar.gz|zip]` assembles one release archive: the binary under `bin/`, the generated completions and man page, and the licenses.
- `fill-manifests.sh <version> <assets-dir> <out-dir>` writes filled copies of the three manifests into an output directory.

## Install

A released `sentil` is self-contained; the Rust toolchain is not needed to run it.

- Homebrew (macOS and Linux): `brew install sedislab/sentil/sentil`
- Scoop (Windows): `scoop bucket add sedislab https://github.com/sedislab/scoop-sentil; scoop install sentil`
- Winget (Windows): `winget install SEDIS.SENTIL`

Each of these puts `sentil` on your `PATH`, so a new shell runs it directly:

```
sentil check -f 'always[0,5] (speed < 30)' -t run.csv
```

For a plain release archive, the ARM and Raspberry Pi builds, or building from source, see the main CLI README linked below.

## How a release fills the manifests

`fill-manifests.sh` finds each staged archive under the assets directory, computes its SHA-256, and rewrites the manifests with the version and those checksums. The names it looks for are the release triples: `sentil-<version>-aarch64-apple-darwin.tar.gz`, `-x86_64-apple-darwin.tar.gz`, and `-x86_64-unknown-linux-gnu.tar.gz`, all gzipped tarballs, and `sentil-<version>-x86_64-pc-windows-msvc.zip`.

The `REPLACE_WITH_*` strings in the checked-in manifests are there on purpose. A checksum only exists once an archive is built, so keeping placeholders means no manifest in the tree can point at a stale hash. The real values live only in the copies a release produces. `fill-manifests.sh` refuses to finish if any placeholder is still present, so a manifest never ships half-filled.

## Where the filled manifests go

Homebrew and Scoop publish by a push, and each only when its token is set. On a tagged release, with `HOMEBREW_TAP_TOKEN` present, the workflow fills the formula and pushes `Formula/sentil.rb` to the tap repository `sedislab/homebrew-sentil`; with `SCOOP_BUCKET_TOKEN` present, it fills the manifest and pushes `bucket/sentil.json` to `sedislab/scoop-sentil`. Both tokens are write access to those repositories, both are optional, and the matching job skips when its token is absent. The release is complete without them, and nothing reaches a distributor until the authors add the secret.

Winget is a central registry (`microsoft/winget-pkgs`), so it takes a pull request rather than a push to a repository the authors own. The workflow stages the filled three-file manifest set as the `winget-manifests` artifact. A maintainer submits it with `wingetcreate submit --token <PAT> winget-manifests/`, or opens the pull request by hand against `manifests/s/SEDIS/SENTIL/<version>/`. There is no Winget repository secret because that submission is the maintainer's step.

The ARM Linux and Raspberry Pi builds, and the CLI `.deb` packages, come from the separate `release-pi` workflow.

For the full tool, see [sentil-cli](../../sentil-cli/README.md). Dual licensed under MIT OR Apache-2.0.