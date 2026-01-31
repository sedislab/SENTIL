# Contributing to SENTIL

Thanks for your interest in SENTIL. This guide covers the ways to contribute, how to set up a development environment for whichever part you want to work on, the house style, and how to open your first pull request. SENTIL runs inside safety-critical real-time systems, so correctness comes first in everything here.

## Ways to contribute

You do not have to touch the Rust core to make a real difference. In rough order of how much of the system you need to hold in your head:

- Documentation and examples: fix a wrong command, clarify a page, add a worked example. No build required for prose; the per-language examples run unmodified.
- A language binding: add or improve the C++, Java, Julia, MATLAB, ROS, or Python surface. Most of these bind the C ABI and need no Rust (see the table below).
- The specifications library: add a vetted, standards-derived PrSTL spec with its citation and a test.
- Benchmarks: add a runner or a baseline comparison in the shared JSON schema.
- The core engine: the robustness semantics, the statistical layer, or the synthesis subsystem in `sentil-core`. This is the deep end and is held to the strictest correctness bar.

If you are unsure where a change belongs, open an issue first and we will point you at the right layer.

## Do you need Rust?

Only the parts that compile Rust need a Rust toolchain. Everything that binds the C ABI can develop against a prebuilt `libsentil`, which you fetch with `scripts/fetch-prebuilt-core.sh` or build once with `cargo build --release -p sentil-ffi`.

| Package | What you are changing | Rust needed |
| --- | --- | --- |
| sentil-core | the engine | yes |
| sentil-ffi | the C ABI | yes |
| sentil-cli | the command-line tool | yes |
| sentil-cpp | the C++ wrapper | no, point at a prebuilt `libsentil` |
| sentil-java | the JNI binding | no, point at a prebuilt `libsentil` |
| sentil-jl | the Julia binding | no, set `SENTIL_LIB` to a prebuilt `libsentil` |
| sentil-matlab | the MATLAB toolbox | no, build the MEX against a prebuilt `libsentil` |
| sentil-ros | the ROS 2 node | no, links the prebuilt C++/C ABI |
| sentil-py | the pure-Python layer, docs, tests | no, install a prebuilt wheel |
| sentil-py | the PyO3 glue in `src/` | yes |
| sentil-embedded | the microcontroller archive | yes, a `no_std` cross-build |

The C ABI packages read the library from `SENTIL_LIB` (Julia) or `SENTIL_LIB_DIR` (the CMake configs), so a contributor with a prebuilt core and no Rust can iterate on the binding layer.

## Development setup

Clone the repository, then set up the piece you are working on. Install instructions differ per operating system; where they do, each package README spells out the Windows, macOS, and Linux path.

Rust, for the core, the C ABI, and the CLI:

```
rustup toolchain install stable
cargo build
cargo test -p sentil
```

A prebuilt core, for a binding you want to develop without Rust:

```
scripts/fetch-prebuilt-core.sh        # downloads libsentil for your platform
```

Python, showing both pip and uv:

```
pip install maturin        # or: uv tool install maturin
cd sentil-py
maturin develop            # builds the extension into the current environment
python -m pytest
```

`cargo build --workspace` and `cargo test --workspace` also build the Python binding's test harness, and that harness links the interpreter rather than the stable ABI the wheel uses. On a stock Linux or macOS install the loader finds `libpython` on its own. If yours lives off the default search path, a conda environment for instance, point the loader at it first:

```
export LD_LIBRARY_PATH="$(python3 -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))'):$LD_LIBRARY_PATH"
```

Each binding's README carries its own build and test commands. The full list of what runs in CI is in `.github/workflows/`.

## House style

SENTIL holds one bar: correctness, then no possible crash, then speed, then a good developer experience, with readability throughout. A few concrete rules the reviewer will check:

- No `unwrap`, `expect`, or `panic!` on any path a user's input can reach. Return a typed error that names the offending construct and where it went wrong.
- `unsafe` is confined to the FFI surface and each block documents the invariant it upholds.
- Comments explain why, not what, and cluster on the genuinely hard parts. The public surface is documented thoroughly with a short runnable example.
- One blank line at most, and every file ends at its last character with no trailing newline.
- Match the surrounding code's naming and idiom. The same concept carries the same word across the core, the C ABI, and every binding.
- Prose, in docs and messages, is plain and direct. No em dashes.

Run the tests and the linter for the package you touched before opening a pull request. For Rust that is `cargo test` and `cargo clippy --all-targets` with no warnings.

## Commits and pull requests

Write commit messages as a person would: mostly a single line in the imperative, explaining the why in a body only when the diff does not already show it. Keep a pull request focused on one change, describe what it does and why, and link the issue it closes. A new public function, type, or behavior needs a test and a doc comment.

## Developer Certificate of Origin

By contributing you certify the Developer Certificate of Origin (https://developercertificate.org): that you wrote the change or have the right to submit it, and that it may be distributed under the project's license. Your contribution is licensed under the same dual terms as SENTIL, MIT or Apache-2.0 at the user's option.

## Your first pull request

1. Fork the repository and create a branch off `main`.
2. Make the change, keeping commits small and focused.
3. Add or update a test, and run the package's tests and linter.
4. Push your branch and open a pull request describing the change.
5. A maintainer reviews it, you address the feedback, and it merges.

If anything here is unclear, open an issue and we will improve this guide.