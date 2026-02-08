# Specifications library

Vetted PrSTL specification templates drawn from standards, textbooks, and papers, so you can reach for a requirement that someone has already read the source for instead of transcribing a clause into temporal logic yourself. Each template is a small TOML file with a plain description of what it captures, the formula in SENTIL's syntax, named parameters with defaults and units, and a citation to the clause or section it comes from.

There are 54 templates across ten domains:

| Domain | Count | Examples |
| --- | --- | --- |
| aerospace | 5 | airspeed envelope, bank angle protection, load factor limit |
| automotive | 5 | safe following distance, time to collision, lane keeping |
| controls | 14 | overshoot, settling time, rise time, steady-state error |
| financial | 4 | max drawdown, volatility bound, circuit-breaker response |
| industrial | 4 | tank level band, pressure relief response, temperature limit |
| medical | 5 | euglycemia band, hypoglycemia recovery, insulin rate limit |
| networking | 4 | latency bound, jitter bound, availability |
| power | 4 | frequency band, low-voltage ride-through, voltage band |
| robotics | 5 | workspace containment, joint limits, obstacle avoidance |
| uav | 4 | geofence, altitude band, battery reserve |

## Using a specification

From the command line, list what is available and inspect one:

```
sentil specs
sentil specs automotive/speed_limit
```

Then check or monitor a trace against it, overriding any parameter whose default does not fit your system. The limits in these files are placeholders where the real value is airframe, route, or patient specific, so set them:

```
sentil check --spec automotive/speed_limit -t run.csv --param speed_limit=27.8 --param T=45
```

A template that carries both a `deterministic` and a `probabilistic` formula selects between them with `--variant`; the probabilistic form pulls in the template's noise model and the statistical settings under `[verification]`.

From Rust, resolve a template through the registry and fill its parameters:

```rust
use sentil::spec_builder::SpecRegistry;

let formula = SpecRegistry::global()
    .builder("automotive/speed_limit")?
    .with_param("speed_limit", 27.8)?
    .with_param("T", 45.0)?
    .build_formula()?;
```

The same library is reachable from every binding under its idiomatic name. To point SENTIL at your own directory of templates instead of the embedded set, set `SENTIL_SPECS_DIR`; a name is resolved there first, then against the built-in library.

## The file format

```toml
[metadata]
name = "Speed Limit"
domain = "automotive"
description = "..."
references = ["ISO 26262-4:2018, Clause 6.4.1"]

[variables]
speed = { unit = "m/s", description = "Vehicle longitudinal ground speed" }

[parameters]
T           = { type = "float", default = 30.0,  unit = "seconds" }
speed_limit = { type = "float", default = 33.3,  unit = "m/s" }
p           = { type = "float", default = 0.95,  range = [0.0, 1.0] }

[formulas]
deterministic = "always[0, {T}](speed <= {speed_limit})"
probabilistic = "P >= {p}(always[0, {T}](speed <= {speed_limit}))"

[noise]
speed = { model = "Gaussian", mean = 0.0, std_dev = 0.2, interaction = "additive" }

[verification.smc]
confidence = 0.95
sample_budget = 1000
```

Parameters are substituted into the formula by name in braces, so `{speed_limit}` becomes the value you pass or the default. The `[noise]`, `[noise_fit]`, and `[verification]` sections are optional and only matter for the probabilistic form.

## Citations

Every reference resolves to a real clause, section, or paper. `references.bib` collects the BibTeX entries the descriptions cite. The wording is paraphrased rather than copied, since the standards themselves are copyrighted; the citation points you at the source if you need the exact text.

## Adding one

Drop a TOML file under the matching domain, give it a `[metadata]` block with a real citation, at least one entry under `[formulas]`, and defaults for every parameter the formula names. The test suite loads every file in this tree and asserts it parses, type-checks, and evaluates on a sample trace, so a malformed template fails the build. Add the BibTeX entry to `references.bib` and verify the formula scores the way the requirement reads on a trace you construct by hand.