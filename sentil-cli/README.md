<div align="center">

# SENTIL

#### The command-line tool for Probabilistic Signal Temporal Logic

[![CLI](https://img.shields.io/badge/CLI-Homebrew%20%7C%20Scoop%20%7C%20Winget-blue.svg)](#install)
[![Website](https://img.shields.io/badge/docs-sentil.pages.dev-blue.svg)](https://sentil.pages.dev)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

The `sentil` command-line tool. It runs the SENTIL engine directly, and it is built for a pipe: data on stdout, logs on stderr, a verdict you can branch on, and a stable JSON contract.

SENTIL is a runtime verification engine for Signal Temporal Logic and its probabilistic extension PrSTL. This tool checks a trace against a formula, monitors a live signal, estimates how likely a probabilistic specification holds, and synthesizes a control input that satisfies one.

## Install

### Package managers

- Homebrew: `brew install sedislab/sentil/sentil`
- Scoop: `scoop bucket add sedislab https://github.com/sedislab/scoop-sentil; scoop install sentil`
- Winget: `winget install SEDIS.SENTIL`

### Prebuilt release

Download the archive for your platform from the [GitHub release](https://github.com/sedislab/SENTIL/releases), unpack it, and put `sentil` on your `PATH`. Each archive also carries the shell completions and the man page. On a Raspberry Pi or other ARM Linux, use the `release-pi` artifacts; `docs/REPRODUCE.md` has the detail.

### Build from source

With the Rust toolchain, clone the repository and install from it:

```bash
git clone https://github.com/sedislab/SENTIL
cd SENTIL
cargo install --path sentil-cli
# or build in place; the binary lands in target/release/sentil
cargo build --release -p sentil-cli
```

## A first look

```bash
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

## Writing a formula

A formula is a predicate over your signals joined with boolean and temporal operators. A predicate compares one signal to a constant with `<`, `<=`, `>`, `>=`, `=`, or `!=`, so `speed < 30` reads the `speed` signal at each step. The monitor reports the quantitative robustness, positive and equal to the margin while the property holds, negative and equal to the shortfall when it fails.

Join predicates with `not`, `and`, `or`, and `implies`, and reach across time with the temporal operators, which come in a future and a past form:

| Operator | Meaning |
| --- | --- |
| `always phi` | `phi` holds at every step from now on |
| `eventually phi` | `phi` holds at some step from now on |
| `phi until psi` | `phi` holds until `psi` becomes true |
| `next phi` | `phi` holds at the next step |
| `historically phi` | `phi` has held at every step so far |
| `once phi` | `phi` has held at some step so far |
| `phi since psi` | `phi` has held since `psi` was last true |

A temporal operator takes an optional window `[a, b]` that bounds it to the interval from `a` to `b` around now, so `always[0, 10] (speed < 30)` checks the next ten seconds and `eventually[0, 5] (gap > 2)` asks the gap to clear within five. The bounded and past forms settle to a verdict from the samples already seen, which suits a live alarm; an unbounded future operator resolves only once its window has passed.

The probabilistic operator `P` turns a formula into a chance constraint. `P>=0.95 (always[0, 10] (gap > 5))` asks whether the inner formula holds with probability at least `0.95` once each reading is lifted into a noise ensemble. Use `P>=`, `P>`, `P<=`, or `P<` with a probability, and describe the sensor noise with `--noise`, or fit it from data with `sentil fit`.

## Traces and input files

A trace is a table: one time column and one column per signal. Each signal binds to the formula variable of the same name, so a formula that reads `speed` needs a `speed` signal, and the tool names the missing one if it is absent. The format is inferred from the content, so no file extension is required, and `-` reads the trace from stdin.

CSV is a header row whose first column is `time`, then one column per signal:

```csv
time,speed,gap
0.0,12.0,8.0
0.1,11.4,7.6
```

JSON is an array of records, each with a `time` key and one key per signal:

```json
[{"time": 0.0, "speed": 12.0, "gap": 8.0},
 {"time": 0.1, "speed": 11.4, "gap": 7.6}]
```

A MATLAB `.mat` file loads the same way, and a `--features formats` build adds Parquet, Arrow, and SQLite. The online `monitor` differs: it reads one JSON record per line from stdin as they arrive rather than a whole file, so a live sensor pipes straight into it.

The premade specifications replace a hand-written formula: `sentil specs` lists them, `sentil specs <name>` inspects one, and `--spec <name>` uses one in place of `-f`.

## The verbs

| Verb | What it does |
| --- | --- |
| `check` | offline robustness of a formula over a trace; `--signal` prints it at every sample, `--violations` the failing intervals |
| `monitor` | the online monitor, stdin to stdout, a verdict per line, a live dashboard on a terminal; a PrSTL formula with `--noise` shows a live probability |
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

Every verb takes the global flags `-o/--output text|json|ndjson`, `--color auto|always|never`, `-q/--quiet`, `--config <FILE>`, and `--no-input`. Run `sentil <verb> --help` for its usage and examples, and the [documentation site](https://sentil.pages.dev) carries the full reference for every command.

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

The two global defaults, the output format and the color policy, each have a config key and a `SENTIL_*` environment variable, so you set them once instead of passing them every run. The sources layer, highest precedence first: a flag on the command line, then the environment variable (`SENTIL_OUTPUT`, `SENTIL_COLOR`), then `./sentil.toml` in the working directory, then the per-user config under your platform's config directory, then `/etc/sentil/config.toml`, then the built-in default. A `sentil.toml` checked into a project travels with it, so a team shares one setup.

A config file is plain TOML:

```toml
output = "json"   # text, json, or ndjson
color = "never"   # auto, always, or never
```

`sentil config` prints which of those files exist and are read, in order, the output and color values in effect, and the aliases defined, so you can see where each setting came from.

## Aliases and plugins

An alias is a named preset for a command line you run often. Define aliases under `[alias]` in `sentil.toml`, the way cargo does, as a single line or as an argument list:

```toml
[alias]
highway = "check --spec automotive/lane_keeping --semantics dense"
glucose = ["smc", "--spec", "medical/glucose", "--samples", "1e5"]
```

When you run `sentil <name>` and `<name>` is not a built-in verb, the tool looks it up in `[alias]`, splits the value with shell quoting honored, and runs it with your extra arguments appended, so `sentil highway -t drive.csv` expands to `sentil check --spec automotive/lane_keeping --semantics dense -t drive.csv`. A built-in verb always wins over an alias of the same name, and `sentil config` lists the aliases in effect.

Plugins add whole new verbs in any language. When `sentil <name>` is neither a built-in nor an alias, the tool searches your `PATH` for an executable named `sentil-<name>` and runs it with the remaining arguments, the way `git` and `cargo` find their subcommands. A `sentil-report` script on the `PATH` runs as `sentil report ...` and receives everything after `report`, so you add a project-specific command without touching the tool.

## Documentation

The [documentation site](https://sentil.pages.dev) carries the full reference for every verb and flag, the formula and specification syntax, worked examples, and the long-form [tutorial](https://sentil.pages.dev/docs/tutorial). At the terminal, `sentil <verb> --help` prints a verb's usage and `sentil explain <operator>` gives an operator's robustness semantics.

## Contributing

Run the tests for a change:

```
cargo test -p sentil-cli
```

The pull-request flow is in the repository [CONTRIBUTING.md](../CONTRIBUTING.md).

## Citation

If SENTIL is useful in your work, please cite the paper:

```bibtex
@misc{quansah2026sentilruntimeverificationtool,
    title={SENTIL: A Runtime Verification Tool for Probabilistic Signal Temporal Logic},
    author={Paapa Kwesi Quansah and Ernest Bonnah},
    year={2026},
    eprint={2605.21676},
    archivePrefix={arXiv},
    primaryClass={cs.LO},
    url={https://arxiv.org/abs/2605.21676}
}
```

## License

SENTIL is by Paapa Kwesi Quansah and Ernest Bonnah at the SEDIS lab, Baylor University. It is dual licensed under either [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE), at your option.