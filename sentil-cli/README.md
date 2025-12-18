# sentil-cli

The `sentil` command-line tool. It runs the SENTIL engine directly, with no library boundary, so it is as fast as the Rust core, and it is built for a pipe: data on stdout, logs on stderr, a verdict you can branch on, and a stable JSON contract.

SENTIL is a runtime verification engine for Signal Temporal Logic and its probabilistic extension PrSTL. This tool checks a trace against a formula, monitors a live signal, estimates how likely a probabilistic specification holds, and synthesizes a control input that satisfies one.

## Install

The released binary is self-contained; you do not need the Rust toolchain to run it.

- Homebrew: `brew install sedislab/sentil/sentil`
- Scoop: `scoop bucket add sedislab https://github.com/sedislab/scoop-sentil; scoop install sentil`
- Winget: `winget install SEDIS.SENTIL`
- GitHub Releases: download the archive for your platform from the releases page, unpack it, and put `sentil` on your `PATH`. Each archive also carries the shell completions and the man page.
- Raspberry Pi and other ARM Linux: see the `release-pi` artifacts and `docs/REPRODUCE.md`.

From source, with the Rust toolchain:

```
cargo install --path sentil-cli
# or, from a clone, the binary lands in target/release/sentil
cargo build --release -p sentil-cli
```

## A first look

```
# offline: did the trace satisfy the formula, and by how much
sentil check -f 'always[0,5] (speed < 30)' -t run.csv

# online: one JSON sample per line in, one verdict per line out
sensor | sentil monitor -f 'always (temp < 80)' -o ndjson | alerter

# statistical: how likely does a probabilistic spec hold
sentil smc -f 'P>=0.95(always[0,10] (x > 0))' -t base.csv --samples 1e5

# synthesis: find a control input that satisfies a spec on a model
sentil synth -f 'always (x > 0)' --model system.json

# falsification: search the input space for a trajectory that breaks the spec
sentil falsify -f 'always (x < 5)' --model system.json

# calibration: fit a noise model from recorded truth and sensor columns
sentil fit -t calib.csv --truth temp_true --sensor temp_meas --model gaussian
```

The trace is CSV (a `time` column then one column per signal) or a JSON array of `{"time": .., "x": ..}` records, and the format is inferred from the content, so no extension is required; pass `-` to read it from stdin. A MATLAB `.mat` file loads too, and Parquet, Arrow, and SQLite with a `--features formats` build. The signal columns bind to the formula's variables by name. The premade specifications replace a hand-written formula: `sentil specs` lists them, `sentil specs <name>` inspects one, and `--spec <name>` uses it.

## The verbs

| Verb | What it does |
| --- | --- |
| `check` | offline robustness of a formula over a trace; `--signal` prints it at every sample, `--violations` the failing intervals |
| `monitor` | the online monitor, stdin to stdout, a verdict per line, a live dashboard on a terminal |
| `smc` | statistical model checking, with `--algo smc\|sprt\|chernoff\|bayes` |
| `synth` | open-loop synthesis from a model file |
| `falsify` | search a model's input space for a trajectory that violates the spec |
| `fit` | fit a noise model from paired ground-truth and sensor columns |
| `mine` | find the tightest spec parameter that still holds on a trace |
| `lift` | apply noise models to a trace, CSV out; `--members N` writes a full ensemble |
| `specs` | list or inspect the premade specifications |
| `explain` | an operator's robustness semantics, or a verb's output fields |
| `config` | the configuration files in effect |
| `completion`, `man` | the shell completion script or the man page |
| `init` | build a check interactively, then print and run the command |

Every verb takes the global flags `-o/--output text|json|ndjson`, `--color auto|always|never`, `-q/--quiet`, `--config <FILE>`, and `--no-input`. Run `sentil <verb> --help` for examples.

## Output and exit codes

stdout carries data, stderr carries progress, logs, and errors, so a redirect keeps a pipe clean. `-o json` emits one self-describing object with a `schema_version`; `-o ndjson` streams one object per line and ends with a `summary` record. Color is on for a terminal and off for a pipe, honoring `NO_COLOR` and `CLICOLOR_FORCE`. A parse error points at the offending token and says what was expected.

The exit code is a verdict you can branch on, so `sentil check ... && deploy` runs only when the spec held:

| Code | Meaning |
| --- | --- |
| 0 | the specification held |
| 10 | the specification was violated (a verdict, not an error) |
| 2 | usage error |
| 65 | bad input data |
| 66 | an input file was not found |
| 69 | a requested backend is unavailable |
| 70 | an internal error |
| 130 | interrupted |

`sentil explain exit-codes` prints this table, and `sentil explain --fields <verb>` prints the JSON a verb emits.

## Configuration

Defaults layer, highest precedence first: a flag, then a `SENTIL_*` environment variable, then `./sentil.toml`, then the per-user config under your config directory, then `/etc/sentil/config.toml`, then the built-in default. `sentil config` shows which files are read and the values in effect. A config file is plain TOML:

```toml
output = "json"
color = "never"
```

## Aliases and plugins

Define your own presets under `[alias]` in `sentil.toml`, the way cargo does. Each is a command line, with shell quoting honored, expanded when the name is not a built-in verb; extra arguments are appended:

```toml
[alias]
highway = "check --spec automotive/lane_keeping --semantics dense"
glucose = "smc --spec medical/glucose --samples 1e5"
```

Then `sentil highway -t drive.csv` runs the lane-keeping check on that trace, and `sentil config` lists the aliases in effect. For a custom subcommand in any language, drop an executable named `sentil-<name>` on your `PATH`; `sentil <name> ...` runs it with the remaining arguments, the way `git` and `cargo` find their plugins.

## Performance

The CLI links the engine as one compilation unit, with no FFI boundary, so it runs at native engine speed. The streaming monitor holds an O(1) amortized per-sample cost with memory proportional to the formula's windows, not the length of the stream.

## Credits and license

SENTIL is the work of Paapa Kwesi Quansah, Ernest Bonnah, and the SEDIS Lab at Baylor University. Dual licensed under MIT or Apache-2.0; see the `LICENSE-MIT` and `LICENSE-APACHE` files at the repository root. The full documentation lives at the SENTIL site linked from the project repository.