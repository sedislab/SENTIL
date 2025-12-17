# Packaging the sentil CLI

The `release-cli` workflow builds the CLI for Linux, macOS (Intel and Apple silicon), and Windows, bundles each binary with its shell completions, man page, and licenses by way of `stage.sh`, and attaches the archives to the GitHub release. The desktop package managers are then updated from those archives. Nothing here publishes on its own; each is a maintainer step once the archives and their checksums exist, in keeping with the project's release model.

The Raspberry Pi and other ARM Linux builds, plus the `.deb` and `.rpm` packages, come from the separate `release-pi` workflow.

## Homebrew

`homebrew/sentil.rb` is the formula. After a release, fill the three `sha256` placeholders with the checksums of the macOS and Linux archives, then open a PR against the tap repository (`sedislab/homebrew-sentil`). Users install with `brew install sedislab/sentil/sentil`.

## Scoop

`scoop/sentil.json` is the manifest. Fill the Windows archive `hash`, then commit it to the bucket repository (`sedislab/scoop-sentil`). Users install with `scoop bucket add sedislab ...; scoop install sentil`. The `autoupdate` block lets Scoop track later releases.

## Winget

`winget/` holds the three-file manifest set for `SEDIS.SENTIL`, a portable zip package. Fill the installer `InstallerSha256`, then submit the set to `microsoft/winget-pkgs` under `manifests/s/SEDIS/SENTIL/1.0.0/`. Users install with `winget install SEDIS.SENTIL`.

## Checksums

Each archive's SHA-256 is what the manifests need. Compute it from the attached release asset, for example `sha256sum sentil-1.0.0-x86_64-apple-darwin.tar.gz`, and paste it into the matching placeholder before submitting.