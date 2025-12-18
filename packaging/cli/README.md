# Packaging the sentil CLI

The `release-cli` workflow builds the CLI for Linux, macOS (Intel and Apple silicon), and Windows, bundles each binary with its shell completions, man page, and licenses by way of `stage.sh`, and attaches the archives to the GitHub release. `fill-manifests.sh` then stamps the distributor manifests with the version and each archive's checksum, and the tap jobs publish from there.

Each tap publishes only when its token is configured, so the release is complete without them and nothing reaches a distributor until the authors add the secret. The Raspberry Pi and other ARM Linux builds, plus the `.deb` and `.rpm` packages, come from the separate `release-pi` workflow.

## Homebrew

`homebrew/sentil.rb` is the formula template. On a tagged release, if the `HOMEBREW_TAP_TOKEN` secret is set, the workflow fills the version and the macOS and Linux checksums and pushes `Formula/sentil.rb` to the tap repository `sedislab/homebrew-sentil`. Users install with `brew install sedislab/sentil/sentil`. Create the tap repository and add a fine-grained PAT with write access to it as `HOMEBREW_TAP_TOKEN` to turn this on.

## Scoop

`scoop/sentil.json` is the manifest template. With `SCOOP_BUCKET_TOKEN` set, the workflow fills the Windows checksum and pushes `bucket/sentil.json` to the bucket repository `sedislab/scoop-sentil`. Users install with `scoop bucket add sedislab https://github.com/sedislab/scoop-sentil; scoop install sentil`.

## Winget

Winget is a central registry (`microsoft/winget-pkgs`), so it takes a pull request rather than a push to an own repository. The workflow stages the three-file manifest set for `SEDIS.SENTIL` with the version and checksum filled in, as the `winget-manifests` artifact. Submit it with `wingetcreate submit --token <PAT> winget-manifests/`, or open the PR by hand against `manifests/s/SEDIS/SENTIL/<version>/`. Users then install with `winget install SEDIS.SENTIL`.

## The tokens, in short

`HOMEBREW_TAP_TOKEN` and `SCOOP_BUCKET_TOKEN` are write tokens for the tap and bucket repositories; both are optional and the matching job skips when absent. Winget needs no repository secret because submission is a maintainer step with a personal token to `wingetcreate`.