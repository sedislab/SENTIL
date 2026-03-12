<div align="center">

# SENTIL

#### The command-line tool for Probabilistic Signal Temporal Logic

[![CLI](https://img.shields.io/badge/CLI-Homebrew%20%7C%20Scoop%20%7C%20Winget-blue.svg)](#install)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

The `sentil` command-line tool, packaged for Homebrew, Scoop, and Winget. It runs the SENTIL engine directly, with no library boundary, so it is as fast as the Rust core.

SENTIL is a runtime verification engine for Signal Temporal Logic and its probabilistic extension PrSTL. This tool checks a trace against a formula, monitors a live signal, estimates how likely a probabilistic specification holds, and synthesizes a control input that satisfies one.

## Install

The released binary is self-contained, so you do not need the Rust toolchain to run it.

### Package managers

- Homebrew: `brew install sedislab/sentil/sentil`
- Scoop: `scoop bucket add sedislab https://github.com/sedislab/scoop-sentil; scoop install sentil`
- Winget: `winget install SEDIS.SENTIL`

### Prebuilt release

Download the archive for your platform from the [GitHub release](https://github.com/sedislab/SENTIL/releases), unpack it, and put `sentil` on your `PATH`. Each archive also carries the shell completions and the man page. On a Raspberry Pi or other ARM Linux, use the `release-pi` artifacts.

### Build from source

With the Rust toolchain:

```
cargo install --path sentil-cli
# or, from a clone, the binary lands in target/release/sentil
cargo build --release -p sentil-cli
```

## Documentation

For the full command reference, the verbs, and the output contract, see [sentil-cli](../../sentil-cli).

## Contributing

The pull-request flow is in the repository [CONTRIBUTING.md](../../CONTRIBUTING.md).

## License

SENTIL is the work of Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab at Baylor University. Dual licensed under MIT OR Apache-2.0; see the `LICENSE-MIT` and `LICENSE-APACHE` files at the repository root.