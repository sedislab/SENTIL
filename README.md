# SENTIL

Runtime verification for probabilistic signal temporal logic.

SENTIL decides whether a stochastic system satisfies a Probabilistic Signal Temporal Logic specification. It computes quantitative robustness over signal traces and estimates satisfaction probability with confidence bounds.

## Reproducing the claims

[docs/CLAIMS.md](docs/CLAIMS.md) lists every performance and correctness number with the command that regenerates it and the tolerance it holds to, and [docs/REPRODUCE.md](docs/REPRODUCE.md) walks through the tiers. On a host without Docker, `make verify` runs the CPU tier in one step; with Docker, `docker compose -f docker/docker-compose.yml up sentil-verify` does the same offline.

Dual licensed under MIT or Apache 2.0, at your option.